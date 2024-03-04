use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use request::{parser, Request};

use dotenv::dotenv;

mod request;
mod router;
mod database;

fn router(request: Request) -> (u16, String) {
    match request.route.as_str() {
        "GET /clientes/:id/extrato" => router::get::bank_statement(request),
        "POST /clientes/:id/transacoes" => router::post::transaction(request),
        _ => router::get::not_found()
    }
}

fn handler(mut client: TcpStream) {
    let mut reader = BufReader::new(&mut client);
    let request = parser(&mut reader).unwrap();

    let (status, body) = router(request);
    
    let response = 
        format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\r\n{body}");
    let _ = client.write(response.as_bytes());
}

fn main() {
    dotenv().ok();

    let listener = TcpListener::bind("0.0.0.0:3000").unwrap();

    for client in listener.incoming() {
        let client = client.unwrap();
        handler(client);
    }
}
