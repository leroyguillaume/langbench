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
from typing import Annotated, Callable
import colorlog

HEADERS = [
    "Language",
    "Elapsed Time (s)",
    "System Time (s)",
    "User Time (s)",
    "CPU Usage (%)",
    "Max Memory (MB)",
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
    max_memory: float
    system_time: float
    user_time: float


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
    docker_build_context: Annotated[
        str | None, Option("--docker-build-context", help="Docker build context")
    ] = None,
    docker_cache: Annotated[
        str | None, Option("--docker-cache", help="Docker cache")
    ] = None,
    docker_registry: Annotated[
        str | None, Option("--docker-registry", help="Docker registry")
    ] = None,
    dry_run: Annotated[bool, Option("--dry-run", help="Dry run")] = False,
    csv_dirpath: Annotated[
        str, Option("--csv-dir", help="Directory to store the CSV results")
    ] = "csv",
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
    push_base_image: Annotated[
        bool, Option("--push-base-image", help="Push the base image")
    ] = False,
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
                        max_memory=float(row[5]),
                        system_time=float(row[2]),
                        user_time=float(row[3]),
                    )
    if not only_render:
        base_tag = "langbench-base"
        logging.info(f"🔨 Building {base_tag} image")
        run_docker_build(
            "base",
            base_tag,
            docker_registry,
            docker_build_context,
            docker_cache,
            {"DATA_SIZE": data_size},
        )
        if docker_registry and push_base_image:
            run(["docker", "push", f"{docker_registry}/{base_tag}"])
        for lang in langs:
            for bench in benchmarks:
                tag = f"langbench-{lang}-{bench}"
                logging.info(f"🔨 Building {tag} image")
                run_docker_build(
                    f"{benchmarks_dir}/{lang}",
                    tag,
                    docker_registry,
                    docker_build_context,
                    docker_cache,
                    target=bench,
                )
                result_dirpath = f"{results_dir}/{lang}/{bench}"
                logging.info(f"🏃 Running benchmark {bench} for {lang}")
                all_elapsed_times = []
                all_system_times = []
                all_user_times = []
                all_cpu_usages = []
                all_max_memory = []
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
                    all_max_memory.append(float(row[4]))

                avg_elapsed_time = round(sum(all_elapsed_times) / num_runs, 2)
                avg_system_time = round(sum(all_system_times) / num_runs, 2)
                avg_user_time = round(sum(all_user_times) / num_runs, 2)
                avg_cpu_usage = int(round(sum(all_cpu_usages) / num_runs))
                avg_max_memory = round(sum(all_max_memory) / num_runs / 1024, 2)

                logging.info(
                    f"📊 Averaged result for {lang} - {bench}: Elapsed={avg_elapsed_time:.2f}s, CPU={avg_cpu_usage}%"
                )
                result = LangResult(
                    cpu_usage=avg_cpu_usage,
                    elapsed_time=avg_elapsed_time,
                    lang=lang,
                    max_memory=avg_max_memory,
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
                            result.max_memory,
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
            sorted_results[SINGLETHREAD_BENCHMARK],
        )
        compare_cpu_table_st = generate_compare_table(
            sorted_results[SINGLETHREAD_BENCHMARK],
            lambda result: result.elapsed_time,
        )
        compare_mem_table_st = generate_compare_table(
            sorted_results[SINGLETHREAD_BENCHMARK],
            lambda result: result.max_memory,
        )
        results_table_mt = generate_results_table(
            sorted_results[MULTITHREADS_BENCHMARK]
        )
        compare_cpu_table_mt = generate_compare_table(
            sorted_results[MULTITHREADS_BENCHMARK],
            lambda result: result.elapsed_time,
        )
        compare_mem_table_mt = generate_compare_table(
            sorted_results[MULTITHREADS_BENCHMARK],
            lambda result: result.max_memory,
        )
        readme = readme_template.render(
            compare_cpu_table_mt=compare_cpu_table_mt,
            compare_cpu_table_st=compare_cpu_table_st,
            compare_mem_table_mt=compare_mem_table_mt,
            compare_mem_table_st=compare_mem_table_st,
            cores=multiprocessing.cpu_count(),
            cpu=platform.processor(),
            data_size=f"{data_size_mb} MB",
            num_runs=num_runs,
            results_table_st=results_table_st,
            results_table_mt=results_table_mt,
            threads=os.cpu_count(),
        )
        with open("README.md", "w") as readme_file:
            readme_file.write(readme)


def generate_compare_table(
    sorted_results: list[LangResult], val: Callable[[LangResult], float]
) -> str:
    html = "<table><tr>"
    html += "<th></th>"
    for result_src in sorted_results:
        html += f"<th>{result_src.lang}</th>"
    html += "</tr>"
    for result_src in sorted_results:
        html += "<tr>"
        html += f"<th>{result_src.lang}</th>"
        for result_tgt in sorted_results:
            val_src = val(result_src)
            val_tgt = val(result_tgt)
            ratio = round(val_src * 100 / val_tgt, 2)
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
        html += f"<td>{result.max_memory}</td>"
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


def run_docker_build(
    context: str,
    tag: str,
    registry: str | None,
    build_context: str | None,
    cache: str | None,
    build_args: dict[str, str] = {},
    target: str | None = None,
) -> str:
    args = ["docker", "build", "-t", tag, "--load"]
    if registry:
        args.extend(["-t", f"{registry}/{tag}"])
    if build_context:
        args.extend(["--build-context", build_context])
    if target:
        args.extend(["--target", target])
    if cache:
        args.extend(
            [
                "--cache-from",
                f"type=local,src={cache}",
                "--cache-to",
                f"type=local,dest={cache}",
            ]
        )
    for key, value in build_args.items():
        args.extend(["--build-arg", f"{key}={value}"])
    args.append(context)
    return run(args)


if __name__ == "__main__":
    typer.run(main)
