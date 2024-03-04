use std::{io::{BufRead, BufReader, Read}, net::TcpStream};

use regex::Regex;
use serde_json::Value;

#[derive(Debug)]
pub struct Request {
    pub verb: String,
    pub account_id: String,
    pub suffix: String,
    pub route: String,
    pub content_length: u64,
    pub body: TransactionInfo
}

#[derive(Debug)]
pub struct TransactionInfo {
    pub valor: String,
    pub tipo: String,
    pub descricao: String
}

impl TransactionInfo {
    fn initialize() -> Self {
        Self {
            valor: String::new(),
            tipo: String::new(),
            descricao: String::new(),
        }
    }
}

pub fn parser(reader: &mut BufReader<&mut TcpStream>) -> Result<Request, Box<dyn std::error::Error>> {
    let mut headline = String::new();
    let _ = reader.read_line(&mut headline)?;

    let headline_pattern = Regex::new(r"^(GET|POST)\s\/clientes\/(\d+)\/(.*?)\sHTTP.*?").unwrap();
    let captures = headline_pattern.captures(&headline).unwrap();

    let mut content_length = 0;
    for line in reader.by_ref().lines() {
        let line = line?;

        if line.starts_with("Content-Length:") {
            // let line_splited: Vec<_> = line.split(":").collect();
            // content_length = line_splited[1].trim().parse::<u64>().unwrap();
            content_length = line.split_once(":")
                     .map(|(_, value)| value.trim().parse::<u64>())
                     .unwrap_or_else(|| Ok(0))?;
            
        }
        if line.is_empty() {
            break;
        }
    }

    let mut body_tsc = TransactionInfo::initialize();

    if content_length > 0 {
        let mut body = String::new();
        let _ = reader.take(content_length).read_to_string(&mut body);

        let parsed: Value = serde_json::from_str(&body)?; 

        let valor =  match parsed.get("valor") {
            Some(valor) => valor,
            None => panic!("Couldn't parse 'valor' field.")
        }.to_string();

        let tipo =  match parsed.get("tipo") {
            Some(tipo) => tipo.as_str().unwrap_or("Error parsing tipo as &str"),
            None => panic!("Couldn't parse 'tipo' field.")
        }.to_string();

        let descricao =  match parsed.get("descricao") {
            Some(descricao) => descricao.as_str().unwrap_or("Error parsing descricsao as &str"),
            None => panic!("Couldn't parse 'descricao' field.")
        }.to_string();

        body_tsc.valor = valor;
        body_tsc.tipo = tipo;
        body_tsc.descricao = descricao;
    }

    let mut req = Request {
        verb: captures.get(1).unwrap().as_str().to_string(),
        account_id: captures.get(2).unwrap().as_str().to_string(),
        suffix: captures.get(3).unwrap().as_str().to_string(),
        route: String::new(),
        content_length: 0,
        body: body_tsc
    };

    req.content_length = content_length;
    req.route = format!("{0} /clientes/:id/{1}", req.verb, req.suffix);
    println!("Request: {req:?}");
    Ok(req)
}