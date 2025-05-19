import csv
import logging
from pathlib import Path
import random
import shutil
import subprocess
from typing import Annotated
import colorlog
from jinja2 import Template
from typer import Argument, Option, Typer

from model import (
    BENCHMARKS,
    BenchmarkMetrics,
    BenchmarkResult,
    BenchmarkResults,
    LanguageResult,
)


ARG_SHORT_COUNT = "-c"
ARG_SHORT_ITERATIONS = "-i"

ARG_LONG_COUNT = "--count"
ARG_LONG_DATA_DIR = "--data-dir"
ARG_LONG_ITERATIONS = "--iterations"
ARG_LONG_LOG_LEVEL = "--log-level"
ARG_LONG_REPORTS_DIR = "--reports-dir"
ARG_LONG_README_TEMPLATE = "--readme-template"
ARG_LONG_REPORT_TEMPLATE = "--report-template"
ARG_LONG_RESULTS_DIR = "--results-dir"

ENV_VAR_COUNT = "LANGBENCH_COUNT"
ENV_VAR_DATA_DIR = "LANGBENCH_DATA_DIR"
ENV_VAR_ITERATIONS = "LANGBENCH_ITERATIONS"
ENV_VAR_LOG_LEVEL = "LANGBENCH_LOG_LEVEL"
ENV_VAR_REPORTS_DIR = "LANGBENCH_REPORTS_DIR"
ENV_VAR_README_TEMPLATE = "LANGBENCH_README_TEMPLATE"
ENV_VAR_REPORT_TEMPLATE = "LANGBENCH_REPORT_TEMPLATE"
ENV_VAR_RESULTS_DIR = "LANGBENCH_RESULTS_DIR"

DEFAULT_COUNT = 25
DEFAULT_DATA_DIR = Path("data")
DEFAULT_ITERATIONS = 1
DEFAULT_LOG_LEVEL = "INFO"
DEFAULT_REPORTS_DIR = Path("reports")
DEFAULT_RESULTS_DIR = Path("results")
DEFAULT_README_TEMPLATE = Path("README.md.j2")
DEFAULT_REPORT_TEMPLATE = Path("report.md.j2")

LANGUAGES_LABEL_MAPPING = {
    "cpp": "c++",
    "csharp": "c#",
}


app = Typer()


class CommandFailedException(Exception):
    def __init__(self, program: str, rc: int, stderr: str):
        self.program = program
        self.return_code = rc
        self.stderr = stderr
        super().__init__(f"Command {program} failed with return code {rc}")


@app.command(help="Generate data files")
def generate_data(
    count: Annotated[
        int,
        Option(
            ARG_SHORT_COUNT,
            ARG_LONG_COUNT,
            envvar=ENV_VAR_COUNT,
            help="The number of integers to generate",
        ),
    ] = DEFAULT_COUNT,
    data_dirpath: Annotated[
        Path,
        Option(
            ARG_LONG_DATA_DIR,
            envvar=ENV_VAR_DATA_DIR,
            help="The output directory",
        ),
    ] = DEFAULT_DATA_DIR,
    iterations: Annotated[
        int,
        Option(
            ARG_SHORT_ITERATIONS,
            ARG_LONG_ITERATIONS,
            envvar=ENV_VAR_ITERATIONS,
            help="The number of iterations",
        ),
    ] = DEFAULT_ITERATIONS,
    log_level: Annotated[
        str,
        Option(
            ARG_LONG_LOG_LEVEL,
            envvar=ENV_VAR_LOG_LEVEL,
            help="The log level",
        ),
    ] = DEFAULT_LOG_LEVEL,
    min: Annotated[
        int,
        Option(
            "--min",
            help="The minimum value",
        ),
    ] = -2147483647,
    max: Annotated[
        int,
        Option(
            "--max",
            help="The maximum value",
        ),
    ] = 2147483647,
):
    __configure_logging(log_level)
    logging.debug(f"📁 Creating output directory ({data_dirpath})")
    data_dirpath.mkdir(parents=True, exist_ok=True)
    for i in range(iterations):
        path = __get_data_filepath(data_dirpath, i)
        logging.info(f"🔄 Generating {count} integers into {path}")
        with open(path, "wb") as file:
            for _ in range(count):
                n = random.randint(min, max)
                file.write(n.to_bytes(4, "little", signed=True))


@app.command(help="Run benchmarks")
def run(
    benchmarks_dirpath: Annotated[
        Path,
        Option(
            "--benchmarks-dir",
            envvar="LANGBENCH_BENCHMARKS",
            help="The benchmarks directory",
        ),
    ] = Path("benchmarks"),
    count: Annotated[
        int,
        Option(
            ARG_SHORT_COUNT,
            ARG_LONG_COUNT,
            envvar=ENV_VAR_COUNT,
            help="The number of integers in each data file",
        ),
    ] = DEFAULT_COUNT,
    data_dirpath: Annotated[
        Path,
        Option(
            ARG_LONG_DATA_DIR,
            envvar=ENV_VAR_DATA_DIR,
            help="The data directory",
        ),
    ] = DEFAULT_DATA_DIR,
    docker_cache: Annotated[
        Path | None,
        Option(
            "--docker-cache",
            envvar="LANGBENCH_DOCKER_CACHE",
            help="The Docker cache directory",
        ),
    ] = None,
    exclude_benchmarks: Annotated[
        list[str],
        Option(
            "--exclude-benchmark",
            envvar="LANGBENCH_EXCLUDE_BENCHMARKS",
            help="The benchmarks to exclude",
        ),
    ] = [],
    exclude_languages: Annotated[
        list[str],
        Option(
            "--exclude-language",
            envvar="LANGBENCH_EXCLUDE_LANGUAGES",
            help="The languages to exclude",
        ),
    ] = [],
    iterations: Annotated[
        int,
        Option(
            ARG_SHORT_ITERATIONS,
            ARG_LONG_ITERATIONS,
            envvar=ENV_VAR_ITERATIONS,
            help="The number of iterations",
        ),
    ] = DEFAULT_ITERATIONS,
    log_level: Annotated[
        str,
        Option(
            ARG_LONG_LOG_LEVEL,
            envvar=ENV_VAR_LOG_LEVEL,
            help="The log level",
        ),
    ] = DEFAULT_LOG_LEVEL,
    no_clean: Annotated[
        bool,
        Option(
            "--no-clean",
            help="Do not clean the temporary directory",
        ),
    ] = False,
    only_benchmarks: Annotated[
        list[str] | None,
        Option(
            "-b",
            "--only-benchmark",
            envvar="LANGBENCH_BENCHMARKS",
            help="The benchmarks to run",
        ),
    ] = None,
    only_languages: Annotated[
        list[str] | None,
        Option(
            "-l",
            "--only-language",
            envvar="LANGBENCH_LANGUAGES",
            help="The languages to run",
        ),
    ] = None,
    readme_tpl_filepath: Annotated[
        Path,
        Option(
            ARG_LONG_README_TEMPLATE,
            envvar=ENV_VAR_README_TEMPLATE,
            help="The README template file",
        ),
    ] = DEFAULT_README_TEMPLATE,
    reports_dirpath: Annotated[
        Path,
        Option(
            ARG_LONG_REPORTS_DIR,
            envvar=ENV_VAR_REPORTS_DIR,
            help="The reports directory",
        ),
    ] = DEFAULT_REPORTS_DIR,
    report_tpl_filepath: Annotated[
        Path,
        Option(
            ARG_LONG_REPORT_TEMPLATE,
            envvar=ENV_VAR_REPORT_TEMPLATE,
            help="The report template file",
        ),
    ] = DEFAULT_REPORT_TEMPLATE,
    results_dirpath: Annotated[
        Path,
        Option(
            ARG_LONG_RESULTS_DIR,
            envvar=ENV_VAR_RESULTS_DIR,
            help="The results directory",
        ),
    ] = DEFAULT_RESULTS_DIR,
    temp_dirpath: Annotated[
        Path,
        Option(
            "--temp-dir",
            envvar="LANGBENCH_TEMP_DIR",
            help="The temporary directory",
        ),
    ] = Path("temp"),
    write: Annotated[
        bool,
        Option(
            "--write",
            help="Write the results",
        ),
    ] = False,
):
    __configure_logging(log_level)
    docker_build_cmd = [
        "docker",
        "build",
        "--load",
    ]
    if docker_cache:
        docker_build_cmd.extend(
            [
                "--cache-from",
                f"type=local,src={docker_cache}",
                "--cache-to",
                f"type=local,dest={docker_cache},mode=max",
            ]
        )
    docker_temp_dirpath_prefix = ""
    if not temp_dirpath.is_absolute():
        docker_temp_dirpath_prefix = "./"
    benchmarks = BENCHMARKS
    results = __load_benchmark_results(results_dirpath)
    if only_benchmarks:
        benchmarks = [
            benchmark for benchmark in BENCHMARKS if benchmark.name in only_benchmarks
        ]
    languages: list[str] = []
    if only_languages:
        languages = only_languages
    elif benchmarks_dirpath.is_dir():
        for language_dirpath in benchmarks_dirpath.iterdir():
            languages.append(language_dirpath.name)
    for language in languages:
        if language in exclude_languages:
            logging.debug(f"⏭️ Skipping language {language} (excluded)")
            continue
        language_dirpath = f"{benchmarks_dirpath}/{language}"
        for benchmark in benchmarks:
            if benchmark.name in exclude_benchmarks:
                logging.debug(f"⏭️ Skipping benchmark {benchmark.name} (excluded)")
                continue
            benchmark_result = results.get_or_create(benchmark.name, count, iterations)
            name = f"langbench-{language}-{benchmark.name}"
            tag = f"langbench-{language}:{benchmark.name}"
            try:
                logging.info(f"🔨 Building image {tag} from context {language_dirpath}")
                __run(
                    docker_build_cmd
                    + [
                        "-t",
                        tag,
                        "--target",
                        benchmark.name,
                        language_dirpath,
                    ]
                )
            except CommandFailedException as err:
                stderr_lines = err.stderr.splitlines()
                if (
                    stderr_lines[-1]
                    == f'ERROR: failed to solve: target stage "{benchmark.name}" could not be found'
                ):
                    logging.warning(f"⚠️  {language} - {benchmark.name} does not exist")
                    continue
                else:
                    __log_command_failed(err)
                    exit(1)
            language_temp_dirpath = temp_dirpath.joinpath(
                f"{language}-{benchmark.name}"
            )
            logging.debug(f"📁 Creating temporary directory {language_temp_dirpath}")
            language_temp_dirpath.mkdir(parents=True, exist_ok=True)
            metrics = BenchmarkMetrics()
            for i in range(iterations):
                data_filepath = __get_data_filepath(data_dirpath, i)
                logging.info(
                    f"🏃 Running {language} - {benchmark.name} ({i + 1}/{iterations})"
                )
                try:
                    __run(
                        [
                            "docker",
                            "rm",
                            "-f",
                            name,
                        ]
                    )
                    __run(
                        [
                            "docker",
                            "run",
                            "--name",
                            name,
                            "--rm",
                            "-e",
                            f"LANGBENCH_DATA_FILE=data/{data_filepath.name}",
                            "-e",
                            f"LANGBENCH_COUNT={count}",
                            "-e",
                            f"LANGBENCH_CORES={benchmark_result.cores}",
                            "-v",
                            f"./{data_dirpath}:/var/lib/langbench/data",
                            "-v",
                            f"{docker_temp_dirpath_prefix}{language_temp_dirpath}:/var/lib/langbench/result",
                            tag,
                        ]
                    )
                except CommandFailedException as err:
                    __log_command_failed(err)
                    exit(1)
                result_filepath = f"{language_temp_dirpath}/result.csv"
                logging.debug(f"📄 Reading results from {result_filepath}")
                with open(result_filepath, "r") as result_file:
                    reader = csv.reader(result_file)
                    row = next(reader)
                    metrics.cpu_usage += int(row[3].removesuffix("%"))
                    metrics.max_memory += int(row[4])
                    metrics.system_time += float(row[1])
                    metrics.time += float(row[0])
                    metrics.user_time += float(row[2])
            result = LanguageResult(
                language=language,
                metrics=metrics.avg(iterations),
            )
            benchmark_result.upsert(result)
            time = round(result.metrics.time, 2)
            cpu_usage = round(result.metrics.cpu_usage, 2)
            max_memory = round(result.metrics.max_memory / 1024, 2)
            logging.info(
                f"📊 {language} - {benchmark.name} results: time={time} cpu={cpu_usage}% memory={max_memory}"
            )
    if write:
        logging.debug(f"📁 Creating directory {results_dirpath}")
        results_dirpath.mkdir(parents=True, exist_ok=True)
        for result in results:
            filepath = f"{results_dirpath}/{result.benchmark.name}_{result.arch}_{result.cores}_{result.count}_{result.iterations}.json"
            logging.debug(f"💾 Saving result to {filepath}")
            result_json = result.model_dump_json()
            with open(filepath, "w") as file:
                file.write(f"{result_json}\n")
        logging.info("💾 Results saved")
    if not no_clean:
        logging.debug(f"🧹 Cleaning directory {temp_dirpath}")
        shutil.rmtree(temp_dirpath)
        logging.info(f"🧹 Directory {temp_dirpath} deleted")


@app.command(help="Read binary file results")
def read(
    path: Annotated[Path, Argument(help="The path the binary file result")],
    log_level: Annotated[
        str,
        Option(
            ARG_LONG_LOG_LEVEL,
            envvar=ENV_VAR_LOG_LEVEL,
            help="The log level",
        ),
    ] = DEFAULT_LOG_LEVEL,
):
    __configure_logging(log_level)
    logging.debug(f"📄 Reading binary results from {path}")
    with open(path, "rb") as file:
        n_bytes = file.read(4)
        while n_bytes:
            n = int.from_bytes(n_bytes, "little", signed=True)
            print(n)
            n_bytes = file.read(4)


@app.command(help="Generate reports")
def render(
    log_level: Annotated[
        str,
        Option(
            ARG_LONG_LOG_LEVEL,
            envvar=ENV_VAR_LOG_LEVEL,
            help="The log level",
        ),
    ] = DEFAULT_LOG_LEVEL,
    reports_dirpath: Annotated[
        Path,
        Option(
            ARG_LONG_REPORTS_DIR,
            envvar=ENV_VAR_REPORTS_DIR,
            help="The reports directory",
        ),
    ] = DEFAULT_REPORTS_DIR,
    readme_tpl_filepath: Annotated[
        Path,
        Option(
            ARG_LONG_README_TEMPLATE,
            envvar=ENV_VAR_README_TEMPLATE,
            help="The README template file",
        ),
    ] = DEFAULT_README_TEMPLATE,
    report_tpl_filepath: Annotated[
        Path,
        Option(
            ARG_LONG_REPORT_TEMPLATE,
            envvar=ENV_VAR_REPORT_TEMPLATE,
            help="The report template file",
        ),
    ] = DEFAULT_REPORT_TEMPLATE,
    results_dirpath: Annotated[
        Path,
        Option(
            ARG_LONG_RESULTS_DIR,
            envvar=ENV_VAR_RESULTS_DIR,
            help="The results directory",
        ),
    ] = DEFAULT_RESULTS_DIR,
):
    __configure_logging(log_level)
    results = __load_benchmark_results(results_dirpath)
    __render(results, reports_dirpath, readme_tpl_filepath, report_tpl_filepath)


def __configure_logging(lvl: str):
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
    logger.setLevel(lvl)


def __get_data_filepath(data_dirpath: Path, iteration: int) -> Path:
    return data_dirpath / f"data_{(iteration + 1):03d}"


def __load_benchmark_results(dirpath: Path) -> BenchmarkResults:
    results = BenchmarkResults()
    if dirpath.is_dir():
        for filepath in dirpath.iterdir():
            logging.debug(f"📄 Loading benchmarks from {filepath}")
            with open(filepath, "r") as file:
                results.append(BenchmarkResult.model_validate_json(file.read()))
    return results


def __load_template(filepath: Path) -> Template:
    logging.debug(f"📄 Loading template from {filepath}")
    with open(filepath, "r") as file:
        return Template(file.read(), keep_trailing_newline=True)


def __log_command_failed(err: CommandFailedException):
    logging.error(f"💥 {err}")
    for line in err.stderr.splitlines():
        logging.error(f"💥 {err.program}: {line}")


def __render(
    results: BenchmarkResults,
    reports_dirpath: Path,
    readme_tpl_filepath: Path,
    report_tpl_filepath: Path,
):
    results.sort()
    readme_tpl = __load_template(readme_tpl_filepath)
    report_tpl = __load_template(report_tpl_filepath)
    if not reports_dirpath.is_dir():
        logging.debug(f"📁 Creating directory {reports_dirpath}")
        reports_dirpath.mkdir(parents=True, exist_ok=True)
    for result in results:
        result.sort()
        filepath = f"{reports_dirpath}/{result.benchmark.name}_{result.arch}_{result.cores}_{result.count}_{result.iterations}.md"
        logging.debug(f"📄 Rendering {filepath}")
        with open(filepath, "w") as file:
            file.write(
                report_tpl.render(
                    languages_label_mapping=LANGUAGES_LABEL_MAPPING,
                    result=result,
                )
            )
    readme_filepath = f"{reports_dirpath}/README.md"
    logging.debug(f"📄 Rendering {readme_filepath}")
    with open(readme_filepath, "w") as file:
        file.write(
            readme_tpl.render(
                results=results,
            )
        )
    logging.info("📈 Reports rendered")


def __run(cmd: list[str]) -> str:
    program = cmd[0]
    logging.debug(f"⚡ Running command: {cmd}")
    process = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout = process.stdout.decode("utf-8").strip()
    stderr = process.stderr.decode("utf-8").strip()
    for line in stdout.splitlines():
        logging.debug(f"{program}: {line}")
    for line in stderr.splitlines():
        logging.debug(f"{program}: {line}")
    if process.returncode != 0:
        raise CommandFailedException(program, process.returncode, stderr)
    return stdout
