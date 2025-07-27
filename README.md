# Ordr

Ordr is a library that can run a set of async functions that depend on each other. For instance if
the result of function `a` and `b` before you can run `c`, order will run `a` and `b` in parallel
before passing their result to `c`.

You can also stop a job midway (or it may stop if a function failed), store whatever results are
already computed, and continue again at another point.

[Full documentation](https://docs.rs/ordr).

The examples may also be useful. If you are missing documentation, let me know.


## Contributing

You are of course welcome to contribute. I don't expect to spend much time on this going forward,
but I'm generally open for suggestions or pull requests.

Tests can be run with

```sh
cargo test --workspace --all-targets
```

I personally use the examples to play around or to test a specific feature.


## License

MIT. But let me know if you do something cool with it. :)
