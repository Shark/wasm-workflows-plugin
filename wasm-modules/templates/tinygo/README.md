# TinyGo

## Development

On macOS, TinyGo is [available on Homebrew](https://github.com/tinygo-org/homebrew-tools):

```
brew tap tinygo-org/tools
brew install tinygo
```

You can then build the module:

```
export GOROOT=$HOME/.asdf/installs/golang/1.17.2/go
tinygo build -wasm-abi=generic -target=wasi -o main.wasm main.go
```
