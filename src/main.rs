use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    process::ExitCode,
    result::Result,
};
use tiny_http::{Method, Request, Response, Server, StatusCode};
use xml::{
    common::{Position, TextPosition},
    reader::{EventReader, XmlEvent},
};

mod lexer;
use lexer::Lexer;

fn parse_entire_xml_file(file_path: &Path) -> Result<String, ()> {
    let file = File::open(file_path).map_err(|err| {
        eprintln!(
            "ERROR: could not open file {file_path}: {err}",
            file_path = file_path.display()
        );
    })?;
    let er = EventReader::new(file);
    let mut content = String::new();
    for event in er.into_iter() {
        let event = event.map_err(|err| {
            let TextPosition { row, column } = err.position();
            let msg = err.msg();
            eprintln!(
                "{file_path}:{row}:{column}: ERROR: {msg}",
                file_path = file_path.display()
            );
        })?;

        if let XmlEvent::Characters(text) = event {
            content.push_str(&text);
            content.push(' ');
        }
    }
    Ok(content)
}

type TermFreq = HashMap<String, usize>;
type TermFreqIndex = HashMap<PathBuf, TermFreq>;

fn check_index(index_path: &str) -> Result<(), ()> {
    println!("Reading {index_path} index file...");

    let index_file = File::open(index_path).map_err(|err| {
        eprintln!("ERROR: could not open index file {index_path}: {err}");
    })?;

    let tf_index: TermFreqIndex = serde_json::from_reader(index_file).map_err(|err| {
        eprintln!("ERROR: could not parse index file {index_path}: {err}");
    })?;

    println!(
        "{index_path} contains {count} files",
        count = tf_index.len()
    );

    Ok(())
}

fn save_tf_index(tf_index: &TermFreqIndex, index_path: &str) -> Result<(), ()> {
    println!("Saving {index_path}...");

    let index_file = File::create(index_path).map_err(|err| {
        eprintln!("ERROR: could not create index file {index_path}: {err}");
    })?;

    serde_json::to_writer(index_file, &tf_index).map_err(|err| {
        eprintln!("ERROR: could not serialize index into file {index_path}: {err}");
    })?;

    Ok(())
}

fn tf_index_of_folder(dir_path: &Path, tf_index: &mut TermFreqIndex) -> Result<(), ()> {
    let dir = fs::read_dir(dir_path).map_err(|err| {
        eprintln!(
            "ERROR: could not open directory {dir_path} for indexing: {err}",
            dir_path = dir_path.display()
        );
    })?;

    'next_file: for file in dir {
        let file = file.map_err(|err| {
            eprintln!(
                "ERROR: could not read next file in directory {dir_path} during indexing: {err}",
                dir_path = dir_path.display()
            );
        })?;

        let file_path = file.path();

        let file_type = file.file_type().map_err(|err| {
            eprintln!(
                "ERROR: could not determine type of file {file_path}: {err}",
                file_path = file_path.display()
            );
        })?;

        if file_type.is_dir() {
            tf_index_of_folder(&file_path, tf_index)?;
            continue 'next_file;
        }

        println!("Indexing {:?}...", &file_path);

        let content = match parse_entire_xml_file(&file_path) {
            Ok(content) => content.chars().collect::<Vec<_>>(),
            Err(()) => continue 'next_file,
        };

        let mut tf = TermFreq::new();

        for token in Lexer::new(&content) {
            if let Some(freq) = tf.get_mut(&token) {
                *freq += 1;
            } else {
                tf.insert(token, 1);
            }
        }

        tf_index.insert(file_path, tf);
    }

    Ok(())
}

fn usage(program: &str) {
    eprintln!("Usage: {program} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands:");
    eprintln!(
        "    index <folder>         index the <folder> and save the index to index.json file"
    );
    eprintln!("    search <index-file>    check how many documents are indexed in the file (searching is not implemented yet)");
    eprintln!("    serve [address]        start local HTTP server with search interface (not implemented yet)");
}

fn serve_static_file(request: Request, file_path: &str, content_type: &str) -> Result<(), ()> {
    let content_type_header =
        tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type).unwrap();

    let file = File::open(file_path).map_err(|err| {
        eprintln!(
            "ERROR: could not open file {file_path}: {err}",
            file_path = file_path
        )
    })?;

    let response = Response::from_file(file).with_header(content_type_header);
    request.respond(response).map_err(|err| {
        eprintln!("ERROR: could not serve static file {file_path}: {err}");
    })
}

fn serve_404(request: Request) -> Result<(), ()> {
    let response = Response::from_string("Not found").with_status_code(StatusCode(404));
    request.respond(response).map_err(|err| {
        eprintln!("ERROR: could not respond to HTTP request: {err}");
    })
}

fn serve_request(mut request: Request) -> Result<(), ()> {
    println!("Received request: {:?}", request);
    match request.method() {
        Method::Post => match request.url() {
            "/api/search" => {
                let mut buf = Vec::new();
                request
                    .as_reader()
                    .read_to_end(&mut buf)
                    .expect("could not read request body");

                let body = std::str::from_utf8(&buf)
                    .map_err(|err| {
                        eprintln!("ERROR: could not parse request body: {err}");
                    })?
                    .chars()
                    .collect::<Vec<_>>();

                println!("Received search query: {body:?}", body = body);
                for token in Lexer::new(&body) {
                    println!("Token: {token:?}", token = token);
                }

                let sample_json_response =
                    serde_json::to_string(&vec!["hello", "world"]).map_err(|err| {
                        eprintln!("ERROR: could not serialize search results: {err}");
                    })?;

                request
                    .respond(Response::from_string(sample_json_response))
                    .map_err(|err| {
                        eprintln!("ERROR: could not respond to HTTP request: {err}");
                    })
            }

            _ => serve_404(request),
        },

        Method::Get => match request.url() {
            "/" | "/index.html" => {
                serve_static_file(request, "index.html", "text/html; charset=utf-8")
            }
            "/index.js" => serve_static_file(request, "index.js", "text/javascript; charset=utf-8"),
            _ => serve_404(request),
        },
        _ => serve_404(request),
    }
}

fn entry() -> Result<(), ()> {
    let mut args = env::args();
    let program = args.next().expect("path to program is provided");

    let subcommand = args.next().ok_or_else(|| {
        usage(&program);
        eprintln!("ERROR: no subcommand is provided");
    })?;

    match subcommand.as_str() {
        "index" => {
            let dir_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no directory is provided for {subcommand} subcommand");
            })?;

            let mut tf_index = TermFreqIndex::new();
            tf_index_of_folder(Path::new(&dir_path), &mut tf_index)?;
            save_tf_index(&tf_index, "index.json")?;
        }

        "search" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no path to index is provided for {subcommand} subcommand");
            })?;

            check_index(&index_path)?;
        }

        "serve" => {
            let address = args.next().unwrap_or("127.0.0.1:8080".to_owned());
            let server = Server::http(&address).map_err(|err| {
                eprintln!("ERROR: could not start HTTP server at {address}: {err}")
            })?;

            println!("Server is listening at http://{address}", address = address);

            for request in server.incoming_requests() {
                serve_request(request)?;
            }
        }

        _ => {
            usage(&program);
            eprintln!("ERROR: unknown subcommand {subcommand}");
            return Err(());
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
