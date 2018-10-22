File Streamer
=============

**WIP**

A quick and easy file sharing solution.

This is a single binary which allows users to quickly drag and drop files to make them available to others via a web interface.

Files are streamed directly from one user to another, and as soon as the sharing user closes the tab, the files are no logner being shared.

Compiling
---------

From scratch, we need to do the following to build the binary ():

```
# First, sort out the client files:
cd client

# Install everything we need to build the client,
# and then build it:
npm install
npm run build

# Next, we compile the server binary:
cd ../server

# Make sure we're using a suitable version of rust (1.30.0 is currently in beta and that's what we want).
# You'll need to install rustup - the rust toolchain management utility - to set up rust.
rustup toolchain install beta
rustup default beta

# Then, compile the binary (which pulls in the client files we just built):
cargo install --path .
```

Running
-------

The above installs our binary (by default to `$HOME/.cargo/bin/file_streamer`).

When run with `--help` it'll list any arguments you can provide.

