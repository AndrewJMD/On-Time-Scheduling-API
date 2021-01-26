#![deny(warnings)]

use warp::Filter;
use std::env;

fn get_server_port() -> u16 {
    env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080)
}

#[tokio::main]
async fn main() {
    let api = filters::events();
    let routes = api.with(warp::log("Events"));

    warp::serve(routes).run(([0,0,0,0], get_server_port())).await;
}

mod filters {
    extern crate redis;
    use redis::Commands;
    use warp::Filter;
    use super::models::{Event};
    use nanoid::nanoid;

    pub fn events() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        new_event()
            .or(post_events())
    }
    
    pub fn new_event() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_credentials(true)
            .allow_headers(vec!["User-Agent", "Access-Control-Allow-Origin", "Access-Control-Allow-Headers", "content-type", "Origin", "Referer", "Access-Control-Request-Method"])
            .allow_methods(vec!["POST", "GET", "HEAD", "OPTIONS"]);
        
        warp::post()
            .and(warp::path!("api" / "v1" / "scheduling" / "events"))
            .and(warp::body::json())
            .map(|event: Event| {
                set_event(&event.organizer, &event.name, &event.date);
                let s = format!("Organizer: {:?} Event: {:?} Date {:?}", event.organizer, event.name, event.date);
                s
            })
            .with(cors)
    }

    fn set_event(org: &str, evt: &str, date: &str) {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut con = client.get_connection().unwrap();
    
        let mut vec = vec![("organizer", org), ("event", evt), ("date", date)];
        let id = &nanoid!();
        vec.push(("id", &id));

        let mut n = 1;
    
        while con.hexists(format!("event:{}", n), "organizer").unwrap() {
            n += 1;
        }
    
        con.hset_multiple(format!("event:{}", n), &vec).unwrap()
    }

    pub fn post_events() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["POST", "GET", "HEAD", "OPTIONS"]);
    
        warp::path!("api" / "v1" / "scheduling" / "events" / "list")
            .map(move || {
                format!("{:?}", get_events())
            })
            .with(cors)

    }

    fn get_events() -> Vec<Vec<String>> {

        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut con = client.get_connection().unwrap();
    
        let mut events = Vec::new();
        let mut n = 1;
    
        while con.hexists(format!("event:{}", n), "organizer").unwrap() {
            println!("Exists");
            events.push(get_event(&mut con, n).unwrap());
            n += 1;
        }
    
        return events
    }
    
    
    fn get_event(con: &mut redis::Connection, n: u8) -> redis::RedisResult<Vec<String>> {
        con.hgetall(format!("event:{}", n))
    }
}

mod models {
    use serde_derive::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct Event {
        pub organizer: String,
        pub name: String,
        pub date: String,
    }

}

