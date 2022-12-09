# It's a Unix System! I know this!

WebAssembly Unix terminal built with 🦀Rust🦀

## Features

* Essential UNIX commands (sh, ls, cp, mv, cat, cowsay, etc)
* Pipes and file redirect
* File system via [rust-vfs](https://github.com/manuel-woelker/rust-vfs)
* Basic scripting support (try `sh example.sh`)
* Basic GNU Readline-like features (key bindings, history, tab-complete)
* Basic ANSI escape code support, including some colors

### Not included (yet)

* File editor

### Known bugs

* Running `sh -c 'echo -- ${2}'` will recurse forever.

## Example

```
$ fortune -s | cowsay
 _____________________________________
< Your mother is disappointed in you. >
 -------------------------------------
         \    ^__^
          \   (oo)\_______
              (__)\       )\/\
                  ||----w |
                  ||     ||
$ # We also have file redirect
$ echo "Wow what a great fortune" > file
$ # We could do `cat file` as well
$ cat < file
Wow what a great fortune
```

## Running locally

* `cargo install wasm-pack`
* `wasm-pack build`
* `cd www`
* `npm install`
* `npm run start`

## License

MIT
