// UNDERSTANDING: The way I understand it but not 100% sure if its right
// PROBLEMS: The thing I don't get

use actix_files as fs;
use actix_session::{CookieSession, Session};
use actix_utils::mpsc;
use actix_web::http::{header, Method, StatusCode};
use actix_web::{
    error, get, guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Result,
};
use std::{env, io};


/// favicon handler
#[get("/favicon")]
async fn favicon() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("../static/favicon.ico")?)
}

/// welcome index handler
#[get("/welcome")]
async fn welcome(session: Session, req: HttpRequest) -> Result<HttpResponse> {
    println!("{:?}", req);
    // Session
    // PROBLEMS: I don't get this session part, it sets counter as cookie what why does it matter in this example?
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("Session value: {}", count);
        counter = count + 1;
    }
    // Set counter to session
    session.set("counter", counter)?;

    // Response and setting HTML headers
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        // UNDERSTANDING: It takes insides of `welcome.html` and passes it to body
        .body(include_str!("../static/welcome.html")))
}

/// Not Found handler
async fn p404() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("../static/404.html")?.set_status_code(StatusCode::NOT_FOUND))
}

/// Response Body
async fn response_body(path: web::Path<String>) -> HttpResponse {
    let text = format!("Hello {}!", *path);

    let (tx, rx_body) = mpsc::channel();
    let _ = tx.send(Ok::<_, Error>(web::Bytes::from(text)));

    HttpResponse::Ok().streaming(rx_body)
}
/// Handler with path parameters lile `/user/{name}`
async fn with_param(req: HttpRequest, web::Path((name,)): web::Path<(String,)>) -> HttpResponse {
    println!("{:?}", req);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Hello {}!", name))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
  env::set_var("RUST_LOG", "actix_web=debug, actix_server=info");
  env_logger::init();

  HttpServer::new(|| 
    {
    App::new()
    // Cookie session middleware
    .wrap(CookieSession::signed(&[0; 32]).secure(false))
    // enable logger - always register actix-web logger middleware last
    .wrap(middleware::Logger::default())
    // register favicon
    .service(favicon)
    // register welcome
    .service(welcome)
    // with path parameters
    .service(web::resource("/user/{name}").route(web::get().to(with_param)))
    // async response body
    .service(
      web::resource("/async-body/{name}").route(web::get().to(response_body)),
  )
  .service(
    web::resource("/test").to(|req: HttpRequest| match *req.method() {
      Method::GET => HttpResponse::Ok(),
      Method::POST => HttpResponse::MethodNotAllowed(),
      _ => HttpResponse::NotFound(),
    })
  )
  .service(web::resource("/error").to(||async {
    error::InternalError::new(
      io::Error::new(io::ErrorKind::Other, "test"), 
      StatusCode::INTERNAL_SERVER_ERROR,
    )
  }))
  // static files
  .service(fs::Files::new("/static", "static").show_files_listing())
  // redirect
  .service(web::resource("/").route(web::get().to(|req: HttpRequest| {
    println!("{:?}",req);
    HttpResponse::Found()
      .header(header::LOCATION, "static/welcome.html")
      .finish()
  })))

  // default
  .default_service(
    // 404 for GET requests
    web::resource("")
      .route(web::get().to(p404))
      // all requests that are not `GET`
      .route(
        web::route()
        .guard(guard::Not(guard::Get()))
        .to(HttpResponse::MethodNotAllowed),
      ),
    )
  })
  .bind("127.0.0.1:4003")?
  .run()
  .await
}
