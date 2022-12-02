# It's a Unix System! I know this!

WebAssembly Unix terminal built with 🦀Rust🦀

## Features

* Essential UNIX commands (sh, ls, cp, mv, cat, cowsay, etc)
* Pipes and file redirect
* File system via [rust-vfs](https://github.com/manuel-woelker/rust-vfs)
* Basic scripting support (try `sh example.sh`)
* Basic GNU Readline-like features (^A, ^E)
* Basic ANSI escape code support, including some colors

### Not included (yet)

* Cursor
* File editor
* Readline history
* Tab completion
* Append-to-file(`>>`)
* Executing of scripts as commands in `/bin`

### Bugs

* `echo -n hello` shows nothing, due to Readline erasing entire line

## Example

```sh
$ fortune -s | cowsay
 ___________________________
< You look beautiful today. >
 ---------------------------
         \    ^__^
          \   (oo)\_______
              (__)\       )\/\
                  ||----w |
                  ||     ||
$ echo "Wow what a great fortune"
Wow what a great fortune
$
```

## Running locally

* `cargo install wasm-pack`
* `wasm-pack build`
* `cd www`
* `npm install`
* `npm run start`

## License

MIT
