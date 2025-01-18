# Lilyenv

Lilyenv is a tool for managing python interpreters and virtualenvs.

## Installation

Lilyenv is written in Rust and can be installed using `cargo install lilyenv`.

## Usage

* `lilyenv activate <project> <version>` will activate a virtualenv. The interpreter will be downloaded and the virtualenv created automatically if needed.
* `lilyenv list` will list all virtualenvs managed by lilyenv. The optional `<project>` argument shows just that project's virtualenvs.
* `lilyenv upgrade <version>` will upgrade the python interpreter to the latest bugfix release.
* `lilyenv set-project-directory <project> <default_directory>?` will set the default directory for the `<project>`. If `<default_directory`> is omitted the current directory is used.
* `lilyenv unset-project-directory <project>` will unset the default directory for the `<project>`.
* `lilyenv set-shell <project>?` allows explicitly setting the shell lilyenv uses when activating a virtualenv. If `<project>` is provided, the shell is only set for that project.
* `lilyenv shell-config` shows shell-specific configuration information. This can be used to set a custom prompt.
* `lilyenv virtualenv <project> <version>` will create a virtualenv for a project using the given python version.
* `lilyenv remove-virtualenv <project> <version>` will delete the specified virtualenv.
* `lilyenv remove-project <project>` will delete all virtualenvs for a project.
* `lilyenv download <version>` will download a python interpreter with the given version.
* `lilyenv download` will list all python interpreters available to download.

## Comparison with other tools

### Pyenv

[`pyenv`](https://github.com/pyenv/pyenv) is a tool for managing Python interpreters.

| Pyenv | Lilyenv |
| --- | --- |
| Compiles each interpreter from source on your machine. | Downloads pre-built binaries. |
| Makes Python interpreters available for use both with and without a virtualenv involved. | Only exposes interpreters via activated virtualenvs. |
| Awkward to update to newer interpreter versions. | Straightforward to update an interpreter with `lilyenv upgrade`. |

### Virtualenvwrapper

| Virtualenvwrapper | Lilyenv |
| --- | --- |
| Works with existing Python interpreters on your system. | Downloads Python interpreters for you. |
| Mostly a collection of shell scripts. | Written in Rust with a small amount of shell for customising the prompt. This can be viewed with the `lilyenv shell-config` command. |
| Requires an existing python interpreter to install. | Installed with Cargo and doesn't require an existing Python interpreter. |
| Uses the [`virtualenv` project](https://virtualenv.pypa.io/en/latest/). | Uses the [built-in `venv` module](https://docs.python.org/3/library/venv.html) from the downloaded interpreter. |
| Provides [many hooks for custom scripting.](https://virtualenvwrapper.readthedocs.io/en/latest/scripts.html#scripts) | Provides opinionated defaults with minimal customisability. |

### Poetry

| Poetry | Lilyenv |
| --- | --- |
| Optimised for project development with only one supported Python version. | Optimised for library development with multiple supported Python versions. |
| Manages dependencies for you. | Stops at providing a virtualenv. |
| Requires an existing python interpreter to install. | Installed with Cargo and doesn't require an existing Python interpreter. |
