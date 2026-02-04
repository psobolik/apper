# apper

Oct 2025

A simple TUI program for launching GUI apps from a Linux terminal. This is exactly like running the GUI app from the 
command line, which may not be all that useful. The spawned app may write a lot of junk to the terminal, and even worse, 
if will be a child of the terminal process, so closing the terminal will also close the GUI app.

It uses desktop files defined by [Freedesktop.org](https://www.freedesktop.org/wiki/Specifications/desktop-entry-spec/).