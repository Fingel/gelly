# Contributing to Gelly

Hey! Thanks for your interest in contributing to Gelly. The following is a set of guidelines
for contributing to the project.

## Reporting Bugs

Issues should be reported to the [Github issue tracker](https://github.com/Fingel/gelly/issues/).
Please include as many relevant details as possible including: steps to reproduce, behavior, 
expected behavior, etc. 

It is possible to turn on Debug logging for Gelly. Please include relevant logs whenever possible.
To turn on debug logs for Flatpak installations run the following command in a terminal:

    RUST_LOG=debug flatpak run io.m51.Gelly
  
For all other installations:

    RUST_LOG=debug gelly

Depending on the issue, it may also be helpful to include logs from the Jellyfin server. 
They can be found udner the admin dashboard -> Logs.
  
## Submitting Changes

Pull requests follow the [standard Github PR flow](https://docs.github.com/en/get-started/using-github/github-flow). 
Please [submit PRs](https://github.com/Fingel/gelly/pulls) against the main branch.

## Development Environment

See the [Development](https://github.com/Fingel/gelly?tab=readme-ov-file#development) section
of the README.

## License

All contributions shall be licensed under the terms of the GPLv3.
