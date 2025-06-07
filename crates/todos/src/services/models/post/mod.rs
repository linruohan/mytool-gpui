use std::env::args;
use std::io::{Read, stdin};

pub mod imp;
pub mod schema;
use crate::establish_connection;
use diesel::ExpressionMethods;
use diesel::OptionalExtension;
use diesel::SqliteConnection;
use diesel::TextExpressionMethods;
use diesel::query_dsl::methods::FilterDsl;
use diesel::query_dsl::methods::FindDsl;
use diesel::query_dsl::methods::LimitDsl;
use diesel::query_dsl::methods::SelectDsl;
use diesel::{RunQueryDsl, SelectableHelper};
use imp::{NewPost, Post};
use schema::posts::dsl::{posts, published, title as dsl_title};
pub fn create_post(conn: &mut SqliteConnection, title: &str, body: &str) -> Post {
    use schema::posts;
    let new_post = NewPost { title, body };

    diesel::insert_into(posts::table)
        .values(&new_post)
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error saving new post")
}

fn get_post() {
    let post_id = args()
        .nth(1)
        .expect("get_post requires a post id")
        .parse::<i32>()
        .expect("Invalid ID");

    let connection = &mut establish_connection();

    let post = posts
        .find(post_id)
        .select(Post::as_select())
        .first(connection)
        .optional(); // This allows for returning an Option<Post>, otherwise it will throw an error

    match post {
        Ok(Some(post)) => println!("Post with id: {} has a title: {}", post.id, post.title),
        Ok(None) => println!("Unable to find post {}", post_id),
        Err(_) => println!("An error occured while fetching post {}", post_id),
    }
}

fn delete_post() {
    let target = args().nth(1).expect("Expected a target to match against");
    let pattern = format!("%{}%", target);

    let connection = &mut establish_connection();
    let num_deleted = diesel::delete(posts.filter(dsl_title.like(pattern)))
        .execute(connection)
        .expect("Error deleting posts");

    println!("Deleted {} posts", num_deleted);
}

fn show_posts() {
    let connection = &mut establish_connection();
    let results: Vec<Post> = posts
        .filter(published.eq(true))
        .limit(5)
        .select(Post::as_select())
        .load(connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{:?}", post);
    }
}

fn write_post() {
    let connection = &mut establish_connection();

    let mut title = String::new();
    let mut body = String::new();

    println!("What would you like your title to be?");
    stdin().read_line(&mut title).unwrap();
    let title = title.trim_end(); // Remove the trailing newline

    println!("\nOk! Let's write {title} (Press {EOF} when finished)\n",);
    stdin().read_to_string(&mut body).unwrap();

    let post = create_post(connection, title, &body);
    println!("\nSaved draft {title} with id {}", post.id);
}

#[cfg(not(windows))]
const EOF: &str = "CTRL+D";

#[cfg(windows)]
const EOF: &str = "CTRL+Z";

fn publish_post() {
    let id = args()
        .nth(1)
        .expect("publish_post requires a post id")
        .parse::<i32>()
        .expect("Invalid ID");
    let connection = &mut establish_connection();

    let post = diesel::update(posts.find(id))
        .set(published.eq(true))
        .returning(Post::as_returning())
        .get_result(connection)
        .unwrap();
    println!("Published post {}", post.title);
}
