# It's a Unix System! I know this!

WebAssembly Unix terminal built with 🦀Rust🦀

## Features

* Essential Unix commands (sh, ls, cp, mv, cat, cowsay, etc)
* Basic Vi implementation
* Pipes and file redirect
* Variables and subshells
* File system via [rust-vfs](https://github.com/manuel-woelker/rust-vfs)
* Basic scripting support (try `sh example.sh`)
* GNU Readline-like features (key bindings, history, tab-complete)
* ANSI escape code support, including some colors

### Known bugs

* Running `foo=bar echo ${foo}` will print `foo`'s old value
* `[` cannot compare multiword values because of how variable substition works
* No emoji support in `vi`

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

## Building and Serving for Development

* `cargo install wasm-pack`
* `wasm-pack build --dev`
* `cd www`
* `npm install`
* `npm run start`

App will be served on port 8080

## Building for Release

* `cargo install wasm-pack`
* `wasm-pack build --no-default-features`
* `cd www`
* `npm install`
* `npm run build`

Build will be in `www/dist/`

## License

MIT
