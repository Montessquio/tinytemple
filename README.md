# tinytemple

A super duper tiny static site generator.

```
Render templates from TOML and Markdown source

Usage: tinytemple [OPTIONS]

Options:
      --sourcedir <SOURCEDIR>  Source directory for template files and content files [default: ./content/]
      --staticdir <STATICDIR>  Source directory for files which will be copied verbatim into the output [default: ./static/]
      --outdir <OUTDIR>        Output directory for rendered HTML [default: ./html/]
      --config <CONFIG>        TOML Configuration file [default: ./tinytemple.toml]
  -h, --help                   Print help
  -V, --version                Print version
```

The generator will take any `*.hbs` file and render it using any variables set in the `CONFIG` toml file.
Rendered files will be output to `OUTDIR`. Files will be copied from `STATICDIR` verbatim into `OUTDIR`,
but will not clobber existing files.