# Lilyenv

Lilyenv is a tool for managing python interpreters and virtualenvs.

## Installation

Lilyenv is written in Rust and can be installed using `cargo install lilyenv`.

## Usage

* `lilyenv activate <project> <version>` will activate a virtualenv.
* `lilyenv list` will list all virtualenvs managed by lilyenv. The optional `<project>` argument shows just that project's virtualenvs.
* `lilyenv upgrade <version>` will upgrade the python interpreter to the latest bugfix release.
* `lilyenv set-project-directory <project> <default_directory>?` will set the default directory for the `<project>`. If `<default_directory`> is omitted the current directory is used.
* `lilyenv unset-project-directory <project>` will unset the default directory for the `<project>`.
* `lilyenv set-shell` allows explicitly setting the shell lilyenv uses when activating a virtualenv.
* `lilyenv shell-config` shows shell-specific configuration information. This can be used to set a custom prompt.
* `lilyenv virtualenv <project> <version>` will create a virtualenv for a project using the given python version.
* `lilyenv remove-virtualenv <project> <version>` will delete the specified virtualenv.
* `lilyenv remove-project <project>` will delete all virtualenvs for a project.
* `lilyenv download <version>` will download a python interpreter with the given version.
* `lilyenv download` will list all python interpreters available to download.
