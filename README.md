# Lilyenv

Lilyenv is a tool for managing python installations and virtualenvs.

## Installation

Lilyenv is written in Rust and can be installed using `cargo install --path .`.

## Usage

`lilyenv list` lists available python interpreters to download.
`lilyenv download <version>` will download a python interpreter with the given version.
`lilyenv virtualenv <version> <project>` will create a virtualenv using the given python version and a project name.
`lilyenv activate <version> <project>` will activate a virtualenv.
