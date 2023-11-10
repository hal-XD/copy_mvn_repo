
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;
use std::fs;
use std::io::{self, BufRead};

#[macro_use]
extern crate lazy_static;
use regex::Regex;
use clap::Parser;
use anyhow::{Result, anyhow, Ok};
use thiserror::Error;

lazy_static! {
    static ref RE: Regex = Regex::new(r"[^\s]+:jar:[^\s]+:").unwrap();
}

#[derive(Debug, Error)]
enum MyError {
    #[error("does not match regex. line=[{}]", .l)]
    NoMatchRegex {l: String},
    #[error("does not match regex. line=[{}]", .l)]
    NoContaineJarPath { l: String},
    #[error("does not match regex. line=[{}]", .l)]
    InvalidFormatLine {l: String},
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// ${HOME}/.m2/repository
    /// marven dependency:treeの実行結果を張り付けたテキストファイル
    #[clap(value_parser)]
    repo_file: std::path::PathBuf,

    /// jarの配置先ディレクトリ
    #[clap(value_parser)]
    target_dir: std::path::PathBuf,
}

/// jar fileへのパスを返す
fn parse(m2_repo_path: &PathBuf,line: String) -> Result<PathBuf> {
    let mut path = PathBuf::from(m2_repo_path);
    if !line.contains("jar") {
        return Err(MyError::NoContaineJarPath{l: line}.into())
    }
    if let Some(s) = RE
            .find(
                line
                .as_str())
                .map(|m| m.as_str().to_string()
            )
    {
        //      group          | artifact     |type| version
        // path com.google.api:api-common:jar:1.10.4
        // m2_repo_path/group/artifact/version/artifact-version.jar
        let p: Vec<&str> = s.split(":").collect();
        if p.len() != 5 {
            return Err(MyError::InvalidFormatLine {l: line}.into());
        }
        let group = p[0].replace(".", "/");
        let artifact = p[1];
        let version = p[3];
        path.push(group);
        path.push(artifact);
        path.push(version);
        path.push(format!("{artifact}-{version}.jar"));
        return Ok(path)
    }
    Err(MyError::NoMatchRegex{l:line}.into())
}

fn _copy_jar(jars: Vec<PathBuf>, target_dir: PathBuf) -> Result<()>{
    for jar_path in jars {
        if jar_path.exists() && jar_path.is_file() {
            let mut dest = target_dir.clone();
            dest.push(jar_path.file_name().unwrap());
            if dest.exists() {
                println!("skip target=[{dest:?}. because jar=[{:?}]] exist.", jar_path.file_name().unwrap());
                continue;
            }
            fs::copy(jar_path, dest)?;
        } else {
            println!("skip... target=[{jar_path:?}]");
        }
    }
    Ok(())
}

fn copy_jar_to_target_dir(m2_repo_path: PathBuf,repo_file:PathBuf, target_dir:PathBuf)
    -> Result<()>
{
    let mut jars = Vec::<PathBuf>::new();
    { // open file
        let file = fs::File::open(repo_file)?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            jars.push(parse(&m2_repo_path, line)?);
        }
    } // close file
    _copy_jar(jars, target_dir)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // $HOME/.m2/repositoryが存在するか確認
    let home_var = std::env::var("HOME")
        .expect("HOME enviroment varivale not set ");
    let mut m2_repo_path = std::path::PathBuf::from(home_var);
    m2_repo_path.push(".m2");
    m2_repo_path.push("repository");
    if !m2_repo_path.exists() || !m2_repo_path.is_dir() {
        eprintln!(".m2 repository direcory doesn't exsit. path={m2_repo_path:?}");
        std::process::exit(1);
    }

    if !args.repo_file.exists() || !args.repo_file.is_file() {
        eprintln!("repo file doesn't exsit. path={:?}", args.repo_file);
        std::process::exit(1);
    }

    if !args.target_dir.exists() || !args.target_dir.is_dir() {
        eprintln!("target dir doesn't exsit. path={:?}", args.repo_file);
        std::process::exit(1);
    }

    if let Err(e) = copy_jar_to_target_dir(
            m2_repo_path,
            args.repo_file,
            args.target_dir
    ) {
        eprintln!("Error occured. error={e}");
        std::process::exit(1);
    }
    Ok(())
}
