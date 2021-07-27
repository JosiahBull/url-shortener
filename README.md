# Url Shortener

A simple REST web-app which allows one to create shortened urls, returning them to the user. When a url is clicked on, it redirects to the original site.

Cloning the project and running `cargo doc --open` is a good way to get an idea of how the code is structured, the commenting is good.

## Build Notes

### Fedora
Install build-essential for Fedora.

`sudo dnf install make automake gcc gcc-c++ kernel-devel`

Don't forget the sqlite library too.

`sudo dnf install libsqlite3x-devel`
### Ubuntu/Debian
The sqlite3 development dependency is required to compile:

`sudo apt-get install libsqlite3-dev`

It is also recommended to install the sqlite cli tool, which is useful for diagnosing problems which may be encountered. Docs are here: https://sqlite.org/cli.html

`sudo apt-get install sqlite3`
### Windows
- VS Build Tools 2017 or later
- sqlite3.dll and sqlite3.lib in project folder (these may need to be compiled using vcpkg)