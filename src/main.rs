use std::{path::PathBuf, io::Write};
use color_eyre::eyre::{Result, bail};
use clap::Parser;
use fs_extra::dir::CopyOptions;
use handlebars::no_escape;
use tracing::{event, Level, span};

/// Render templates from TOML and Markdown source
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Source directory for template files and content files.
    #[arg(long, default_value = "./content/")]
    sourcedir: PathBuf,

    /// Source directory for files which will be copied verbatim into the output.
    #[arg(long, default_value = "./static/")]
    staticdir: PathBuf,

    /// Output directory for rendered HTML.
    #[arg(long, default_value = "./html/")]
    outdir: PathBuf,

    /// TOML Configuration file.
    #[arg(long, default_value = "./tinytemple.toml")]
    config: PathBuf,
}

type Context = toml::Table;

fn main() -> Result<()> {
    use std::time::Instant;
    let now = Instant::now();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();

    let mut ctx: Context = match std::fs::read_to_string(&args.config) {
        Ok(raw) => match toml::from_str(&raw) {
            Ok(cfg) => cfg,
            Err(e) => {
                let infile = args.config.as_os_str().to_string_lossy();
                event!(Level::ERROR, path = %infile, error = %e, "Unable to parse config file.");
                bail!("A fatal error has occurred.");
            }
        },
        Err(e) => {
            let infile = args.config.as_os_str().to_string_lossy();
            event!(Level::ERROR, path = %infile, error = %e, "Unable to read config file.");
            bail!("A fatal error has occurred.");
        }
    };

    match std::fs::read_dir(&args.sourcedir) {
        Ok(_) => (),
        Err(e) => {
            let id = args.sourcedir.as_os_str().to_string_lossy();
            event!(Level::ERROR, path = %id, error = %e, "Unable to read input directory.");
            bail!("A fatal error has occurred.");
        }
    };

    // First wipe out the old output directory.
    match std::fs::remove_dir_all(&args.outdir) {
        Ok(_) => (),
        Err(e) => {
            let od = args.outdir.as_os_str().to_string_lossy();
            event!(Level::ERROR, path = %od, error = %e, "Unable to clear output directory.");
            bail!("A fatal error has occurred.");
        }
    };
    // Recreate it for use.
    match std::fs::create_dir_all(&args.outdir) {
        Ok(_) => (),
        Err(e) => {
            let od = args.outdir.as_os_str().to_string_lossy();
            event!(Level::ERROR, path = %od, error = %e, "Unable to create output directory.");
            bail!("A fatal error has occurred.");
        }
    };

    // Now read all the source files, apply the context, render, and output.
    let mut engine = handlebars::Handlebars::new();
    engine.register_escape_fn(no_escape);
    match engine.register_templates_directory(".hbs", &args.sourcedir) {
        Ok(_) => (),
        Err(e) => {
            let id = args.sourcedir.as_os_str().to_string_lossy();
            event!(Level::ERROR, path = %id, error = %e, "Unable to parse input templates.");
            bail!("A fatal error has occurred.");
        }
    };

    // Next render every template in sequence.
    for name in engine.get_templates().keys() {
        let _span = span!(Level::INFO, "render_template", template = %name).entered();
        
        // Render markdown, if there is any.
        let mut content_file = args.sourcedir.clone();
        content_file.push(format!("{name}.md"));
        if content_file.exists() {
            match std::fs::read_to_string(&content_file) {
                Ok(raw) => {
                    let parse_opts = pulldown_cmark::Options::all();
                    let parser = pulldown_cmark::Parser::new_ext(&raw, parse_opts);
                    let mut html_output = String::new();
                    pulldown_cmark::html::push_html(&mut html_output, parser);
                    ctx.insert("content".to_owned(), toml::Value::String(html_output));
                },
                Err(e) => {
                    let infile = content_file.as_os_str().to_string_lossy();
                    event!(Level::ERROR, path = %infile, error = %e, "Unable to read content file.");
                }
            };
        }
        else {
            ctx.remove("content");
        }

        // Render the template.
        let mut outfile = args.outdir.clone();
        outfile.push(format!("{name}.html"));

        let parentdir = match outfile.parent() {
            Some(p) => p,
            None => {
                let dir = outfile.as_os_str().to_string_lossy();
                event!(Level::ERROR, path = %dir, "Error manipulating output directory.");
                continue;
            }
        };

        match std::fs::create_dir_all(parentdir) {
            Ok(_) => (),
            Err(e) => {
                let id = args.sourcedir.as_os_str().to_string_lossy();
                event!(Level::ERROR, path = %id, error = %e, "Unable to create output subdirectory.");
                continue;
            }
        };



        match engine.render(name, &ctx) {
            Ok(rendered) => match std::fs::File::create(&outfile) {
                Ok(mut fd) => match write!(fd, "{rendered}") {
                    Ok(_) => (),
                    Err(e) => {
                        let outfile = outfile.as_os_str().to_string_lossy();
                        event!(Level::ERROR, path = %outfile, error = %e, "Error writing to output file.");
                    }
                },
                Err(e) => {
                    let outfile = content_file.as_os_str().to_string_lossy();
                    event!(Level::ERROR, path = %outfile, error = %e, "Error creating output file.");
                }
            },
            Err(e) => {
                let infile = content_file.as_os_str().to_string_lossy();
                event!(Level::ERROR, path = %infile, error = %e, "Error rendering template.");
            }
        }

        let _ = _span.exit();
    }


    // Last, copy the static directory's contents into the output directory
    let copy_res = fs_extra::dir::copy(&args.staticdir, &args.outdir, &CopyOptions {
        overwrite: false,
        skip_exist: false,
        copy_inside: false,
        content_only: true,
        buffer_size: 64000,
        depth: 0,
    });

    match copy_res {
        Ok(_) => (),
        Err(e) => {
            event!(Level::ERROR, error = %e, "Unable to copy static files to output.");
            bail!("A fatal error has occurred.");
        }
    }

    let elapsed = now.elapsed();
    println!("Finished. ({:.2?})", elapsed);

    Ok(())
}