#!/usr/bin/env python3

import csv
from dataclasses import dataclass
from jinja2 import Template
import logging
import multiprocessing
import os
import platform
import subprocess
import typer
from typer import Option
from typing import Annotated
import colorlog

HEADERS = [
    "Language",
    "Elapsed Time (s)",
    "System Time (s)",
    "User Time (s)",
    "CPU Usage (%)",
]

SINGLETHREAD_BENCHMARK = "singlethread"
MULTITHREADS_BENCHMARK = "multithreads"
BENCHMARKS = [SINGLETHREAD_BENCHMARK, MULTITHREADS_BENCHMARK]

app = typer.Typer()


@dataclass
class LangResult:
    cpu_usage: int
    elapsed_time: float
    lang: str
    system_time: float
    user_time: float

    def to_list(self) -> list[str]:
        return [
            self.lang,
            self.elapsed_time,
            self.cpu_usage,
            self.system_time,
            self.user_time,
        ]


@app.command()
def main(
    benchmarks: Annotated[
        list[str], Option("-b", "--benchmarks", help="Benchmarks to run")
    ] = BENCHMARKS,
    benchmarks_dir: Annotated[
        str, Option("--benchmarks-dir", help="Directory containing the benchmarks")
    ] = "benchmarks",
    data_size: Annotated[
        int, Option("--data-size", help="Size of the data to generate")
    ] = 140000000,
    dry_run: Annotated[bool, Option("--dry-run", help="Dry run")] = False,
    langs: Annotated[
        list[str], Option("-l", "--lang", help="Language to run the benchmarks for")
    ] = [],
    log_level: Annotated[str, Option("--log-level", help="Log level")] = "INFO",
    num_runs: Annotated[
        int, Option("--num-runs", help="Number of times to run each benchmark")
    ] = 3,
    only_render: Annotated[
        bool, Option("--only-render", help="Only render the README")
    ] = False,
    csv_dirpath: Annotated[
        str, Option("--csv-dir", help="Directory to store the CSV results")
    ] = "csv",
    readme_template_filepath: Annotated[
        str, Option("--readme-template", help="Filepath to the README template")
    ] = "README.md.j2",
    results_dir: Annotated[
        str, Option("--results-dir", help="Directory to store the results")
    ] = "results",
) -> None:
    handler = colorlog.StreamHandler()
    handler.setFormatter(
        colorlog.ColoredFormatter(
            "%(log_color)s%(message)s",
            log_colors={
                "DEBUG": "cyan",
                "INFO": "green",
                "WARNING": "yellow",
                "ERROR": "red",
                "CRITICAL": "red,bg_white",
            },
        )
    )
    logger = colorlog.getLogger()
    logger.addHandler(handler)
    logger.setLevel(log_level)
    if len(langs) == 0:
        langs = [dir for dir in os.listdir(benchmarks_dir)]
    results = {
        SINGLETHREAD_BENCHMARK: {},
        MULTITHREADS_BENCHMARK: {},
    }
    if os.path.isdir(csv_dirpath):
        logging.debug("📄 Loading existing results")
        for csv_filepath in os.listdir(csv_dirpath):
            bench = csv_filepath.removesuffix(".csv")
            with open(f"{csv_dirpath}/{csv_filepath}", "r") as result_file:
                reader = csv.reader(result_file)
                next(reader, None)
                for row in reader:
                    results[bench][row[0]] = LangResult(
                        cpu_usage=int(row[4]),
                        elapsed_time=float(row[1]),
                        lang=row[0],
                        system_time=float(row[2]),
                        user_time=float(row[3]),
                    )
    if not only_render:
        logging.info("🔨 Building base image")
        run(
            [
                "docker",
                "build",
                "--build-arg",
                f"DATA_SIZE={data_size}",
                "-t",
                "langbench-base",
                "base",
            ]
        )
        for lang in langs:
            for bench in benchmarks:
                tag = f"langbench-{lang}-{bench}"
                logging.info(f"🔨 Building {tag} image")
                run(
                    [
                        "docker",
                        "build",
                        "--target",
                        bench,
                        "-t",
                        tag,
                        f"{benchmarks_dir}/{lang}",
                    ]
                )
                result_dirpath = f"{results_dir}/{lang}/{bench}"
                logging.info(f"🏃 Running benchmark {bench} for {lang}")
                all_elapsed_times = []
                all_system_times = []
                all_user_times = []
                all_cpu_usages = []

                for i in range(num_runs):
                    logging.info(f"🔄 Run {i + 1}/{num_runs} for {lang} - {bench}")
                    value = run(
                        [
                            "docker",
                            "run",
                            "-v",
                            f"./{result_dirpath}:/var/lib/langbench",
                            tag,
                        ]
                    )

                    logging.info(f"🧮 Result: {value}")
                    result_filepath = f"{result_dirpath}/result.csv"
                    logging.debug(
                        f"📄 Loading results from {result_filepath} for run {i + 1}"
                    )
                    with open(result_filepath, "r") as result_file:
                        reader = csv.reader(result_file)
                        row = next(reader)
                    all_elapsed_times.append(float(row[0]))
                    all_system_times.append(float(row[1]))
                    all_user_times.append(float(row[2]))
                    all_cpu_usages.append(int(row[3].removesuffix("%")))

                avg_elapsed_time = sum(all_elapsed_times) / num_runs
                avg_system_time = sum(all_system_times) / num_runs
                avg_user_time = sum(all_user_times) / num_runs
                avg_cpu_usage = int(round(sum(all_cpu_usages) / num_runs))

                logging.info(
                    f"📊 Averaged result for {lang} - {bench}: Elapsed={avg_elapsed_time:.2f}s, CPU={avg_cpu_usage}%"
                )
                result = LangResult(
                    cpu_usage=avg_cpu_usage,
                    elapsed_time=avg_elapsed_time,
                    lang=lang,
                    system_time=avg_system_time,
                    user_time=avg_user_time,
                )
                results[bench][lang] = result
    if not dry_run:
        sorted_results = {}
        if not os.path.isdir(csv_dirpath):
            logging.debug(f"📂 Creating directory {csv_dirpath}")
            os.makedirs(csv_dirpath)
        for bench in BENCHMARKS:
            sorted_results[bench] = sorted(
                results[bench].values(), key=lambda x: x.elapsed_time
            )
            output_filepath = f"{csv_dirpath}/{bench}.csv"
            logging.debug(f"📝 Writing results to {output_filepath}")
            with open(output_filepath, "w") as output_file:
                writer = csv.writer(output_file)
                writer.writerow(HEADERS)
                for result in sorted_results[bench]:
                    writer.writerow(
                        [
                            result.lang,
                            result.elapsed_time,
                            result.system_time,
                            result.user_time,
                            result.cpu_usage,
                        ]
                    )
        logging.debug("📖 Loading README template")
        with open(readme_template_filepath, "r") as readme_template_file:
            readme_template = Template(
                readme_template_file.read(), keep_trailing_newline=True
            )
        logging.debug("📝 Writing README.md")
        data_size_mb = round(data_size * 4 / 1024 / 1024)
        results_table_st = generate_results_table(
            sorted_results[SINGLETHREAD_BENCHMARK]
        )
        compare_table_st = generate_compare_table(
            sorted_results[SINGLETHREAD_BENCHMARK]
        )
        results_table_mt = generate_results_table(
            sorted_results[MULTITHREADS_BENCHMARK]
        )
        compare_table_mt = generate_compare_table(
            sorted_results[MULTITHREADS_BENCHMARK]
        )
        readme = readme_template.render(
            compare_table_st=compare_table_st,
            compare_table_mt=compare_table_mt,
            cores=multiprocessing.cpu_count(),
            cpu=platform.processor(),
            data_size=f"{data_size_mb} MB",
            results_table_st=results_table_st,
            results_table_mt=results_table_mt,
            threads=os.cpu_count(),
        )
        with open("README.md", "w") as readme_file:
            readme_file.write(readme)


def generate_compare_table(sorted_results: list[LangResult]) -> str:
    html = "<table><tr>"
    html += "<th></th>"
    for result_src in sorted_results:
        html += f"<th>{result_src.lang}</th>"
    html += "</tr>"
    for result_src in sorted_results:
        html += "<tr>"
        html += f"<th>{result_src.lang}</th>"
        for result_tgt in sorted_results:
            ratio = round(result_src.elapsed_time * 100 / result_tgt.elapsed_time, 2)
            html += f"<td>{ratio}%</td>"
        html += "</tr>"
    html += "</table>"
    return html


def generate_results_table(sorted_results: list[LangResult]) -> str:
    html = "<table><tr>"
    for header in HEADERS:
        html += f"<th>{header}</th>"
    html += "</tr>"
    for result in sorted_results:
        html += "<tr>"
        html += f"<td>{result.lang}</td>"
        html += f"<td>{result.elapsed_time}</td>"
        html += f"<td>{result.system_time}</td>"
        html += f"<td>{result.user_time}</td>"
        html += f"<td>{result.cpu_usage}</td>"
        html += "</tr>"
    html += "</table>"
    return html


def run(cmd: list[str]) -> str:
    program = cmd[0]
    logging.debug(f"⚡ Running command: {cmd}")
    process = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout = process.stdout.decode("utf-8").strip()
    stderr = process.stderr.decode("utf-8").strip()
    for line in stdout.splitlines():
        logging.debug(f"{program}: {line}")
    if process.returncode == 0:
        for line in stderr.splitlines():
            logging.debug(f"{program}: {line}")
    else:
        logging.error(
            f"💥 Command {program} failed with return code {process.returncode}"
        )
        for line in stderr.splitlines():
            logging.error(f"💥 {program}: {line}")
        exit(process.returncode)
    return stdout


if __name__ == "__main__":
    typer.run(main)
