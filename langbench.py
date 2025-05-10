#!/usr/bin/env python3

from jinja2 import Template
import logging
import os
import pandas
import subprocess
import typer
from typer import Option
from typing import Annotated
import colorlog

app = typer.Typer()


@app.command()
def main(
    benchmarks_dir: Annotated[
        str, Option("--benchmarks-dir", help="Directory containing the benchmarks")
    ] = "benchmarks",
    data_size: Annotated[
        int, Option("--data-size", help="Size of the data to generate")
    ] = 40000000,
    dry_run: Annotated[bool, Option("--dry-run", help="Dry run")] = False,
    langs: Annotated[list[str], Option(help="Language to run the benchmarks for")] = [],
    log_level: Annotated[str, Option("--log-level", help="Log level")] = "INFO",
    output_filepath: Annotated[
        str, Option("-o", "--output", help="Filepath to store the results")
    ] = "results.csv",
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
    results = None
    for lang in langs:
        tag = f"langbench-{lang}"
        logging.info(f"🔨 Building {tag} image")
        run(["docker", "build", "-t", tag, f"{benchmarks_dir}/{lang}"])
        logging.info(f"🏃 Running benchmarks for {lang}")
        run(["docker", "run", "-v", f"./{results_dir}/{lang}:/var/lib/langbench", tag])
        result = pandas.read_csv(
            f"{results_dir}/{lang}/result.csv",
            names=["Elapsed Time", "System Time", "User Time", "CPU Usage"],
        )
        result.insert(0, "Language", lang)
        if results is None:
            results = result
        else:
            results = pandas.concat([results, result], ignore_index=True)
    if not dry_run:
        results.sort_values(by=["Elapsed Time"], inplace=True)
        results.to_csv(output_filepath, index=False)
        table = results.to_markdown(index=False)
        logging.debug("📖 Loading README template")
        with open(readme_template_filepath, "r") as readme_template_file:
            readme_template = Template(
                readme_template_file.read(), keep_trailing_newline=True
            )
        logging.debug("📝 Writing README.md")
        data_size_mb = round(data_size * 4 / 1024 / 1024)
        readme = readme_template.render(data_size=f"{data_size_mb} MB", table=table)
        with open("README.md", "w") as readme_file:
            readme_file.write(readme)


def run(cmd: list[str]):
    program = cmd[0]
    logging.debug(f"⚡ Running command: {cmd}")
    process = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout = process.stdout.decode("utf-8")
    stderr = process.stderr.decode("utf-8")
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


if __name__ == "__main__":
    typer.run(main)
