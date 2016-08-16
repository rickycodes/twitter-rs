//! Structs and methods for pulling user information from Twitter.
//!
//! All the functions in this module eventually return either a [TwitterUser][] struct or the
//! numeric ID of one. The TwitterUser struct itself contains many fields, relating to the user's
//! profile information and a handful of UI settings available to them. See the struct's
//! documention for details.
//!
//! [TwitterUser]: struct.TwitterUser.html
//!
//! ## `UserCursor`/`UserLoader` and `IDCursor`/`IDLoader` (and `UserSearch`)
//!
//! The functions that return the \*Loader structs all return paginated results, implemented over
//! the network as the corresponding \*Cursor structs. The Loader structs both implement
//! `Iterator`, returning an individual user or ID at a time. This allows them to easily be used
//! with regular iterator adaptors and looped over:
//!
//! ```rust,no_run
//! # let consumer_token = egg_mode::Token::new("", "");
//! # let access_token = egg_mode::Token::new("", "");
//! for user in egg_mode::user::friends_of("rustlang", &consumer_token, &access_token)
//!                            .with_page_size(5)
//!                            .map(|resp| resp.unwrap().response)
//!                            .take(5) {
//!     println!("{} (@{})", user.name, user.screen_name);
//! }
//! ```
//!
//! The actual Item returned by the iterator is `Result<Response<TwitterUser>, Error>`; rate-limit
//! information and network errors are passed into the loop as-is.

use std::borrow::Borrow;
use std::collections::HashMap;
use common::*;
use error;
use auth;
use links;

mod structs;

pub use user::structs::*;

///Lookup a set of Twitter users by their numerical ID.
pub fn lookup_ids(ids: &[i64], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = ids.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",");
    add_param(&mut params, "user_id", id_param);

    let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup a set of Twitter users by their screen name.
pub fn lookup_names<S: Borrow<str>>(names: &[S], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = names.join(",");
    add_param(&mut params, "screen_name", id_param);

    let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup a set of Twitter users by both ID and screen name, as applicable.
pub fn lookup(accts: &[UserID], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = accts.iter()
                        .filter_map(|x| match x {
                            &UserID::ID(id) => Some(id.to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(",");
    let name_param = accts.iter()
                          .filter_map(|x| match x {
                              &UserID::ScreenName(name) => Some(name),
                              _ => None,
                          })
                          .collect::<Vec<_>>()
                          .join(",");

    add_param(&mut params, "user_id", id_param);
    add_param(&mut params, "screen_name", name_param);

    let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup user information for a single user.
pub fn show<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::get(links::users::SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup relationship settings between two arbitrary users.
pub fn relation<'a, F, T>(from: F, to: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Relationship>, error::Error>
    where F: Into<UserID<'a>>,
          T: Into<UserID<'a>>
{
    let mut params = HashMap::new();
    match from.into() {
        UserID::ID(id) => add_param(&mut params, "source_id", id.to_string()),
        UserID::ScreenName(name) => add_param(&mut params, "source_screen_name", name),
    };
    match to.into() {
        UserID::ID(id) => add_param(&mut params, "target_id", id.to_string()),
        UserID::ScreenName(name) => add_param(&mut params, "target_screen_name", name),
    };

    let mut resp = try!(auth::get(links::users::FRIENDSHIP_SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup users based on the given search term.
pub fn search<'a>(query: &'a str, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> UserSearch<'a>
{
    UserSearch::new(query, con_token, access_token)
}

///Lookup the users a given account follows, also called their "friends" within the API.
///
///This function returns an iterator over the `TwitterUser` objects returned by Twitter. This
///method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn friends_of<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> CursorIter<'a, UserCursor>
{
    CursorIter::new(links::users::FRIENDS_LIST, con_token, access_token, Some(acct.into()), Some(20))
}

///Lookup the users a given account follows, also called their "friends" within the API, but only
///return their user IDs.
///
///This function returns an iterator over the User IDs returned by Twitter. This method defaults to
///returning 500 IDs in a single network call; the maximum is 5000.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
pub fn friends_ids<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> CursorIter<'a, IDCursor>
{
    CursorIter::new(links::users::FRIENDS_IDS, con_token, access_token, Some(acct.into()), Some(500))
}

///Lookup the users that follow a given account.
///
///This function returns an iterator over the `TwitterUser` objects returned by Twitter. This
///method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn followers_of<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> CursorIter<'a, UserCursor>
{
    CursorIter::new(links::users::FOLLOWERS_LIST, con_token, access_token, Some(acct.into()), Some(20))
}

///Lookup the users that follow a given account, but only return their user IDs.
///
///This function returns an iterator over the User IDs returned by Twitter. This method defaults to
///returning 500 IDs in a single network call; the maximum is 5000.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
pub fn followers_ids<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> CursorIter<'a, IDCursor>
{
    CursorIter::new(links::users::FOLLOWERS_IDS, con_token, access_token, Some(acct.into()), Some(500))
}

///Lookup the users that have been blocked by the authenticated user.
///
///Note that while loading a user's blocks list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn blocks<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, UserCursor> {
    CursorIter::new(links::users::BLOCKS_LIST, con_token, access_token, None, None)
}

///Lookup the users that have been blocked by the authenticated user, but only return their user
///IDs.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
///
///Note that while loading a user's blocks list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn blocks_ids<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, IDCursor> {
    CursorIter::new(links::users::BLOCKS_IDS, con_token, access_token, None, None)
}

///Lookup the users that have been muted by the authenticated user.
///
///Note that while loading a user's mutes list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn mutes<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, UserCursor> {
    CursorIter::new(links::users::MUTES_LIST, con_token, access_token, None, None)
}

///Lookup the users that have been muted by the authenticated user, but only return their user IDs.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
///
///Note that while loading a user's mutes list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn mutes_ids<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, IDCursor> {
    CursorIter::new(links::users::MUTES_IDS, con_token, access_token, None, None)
}

///Lookup the user IDs who have pending requests to follow the authenticated protected user.
///
///If the authenticated user is not a protected account, this will return an empty collection.
pub fn incoming_requests<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> CursorIter<'a, IDCursor>
{
    CursorIter::new(links::users::FRIENDSHIPS_INCOMING, con_token, access_token, None, None)
}

///Lookup the user IDs with which the authenticating user has a pending follow request.
pub fn outgoing_requests<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> CursorIter<'a, IDCursor>
{
    CursorIter::new(links::users::FRIENDSHIPS_OUTGOING, con_token, access_token, None, None)
}

///Lookup the user IDs that the authenticating user has disabled retweets from.
///
///Use `update_follow` to enable/disable viewing retweets from a specific user.
pub fn friends_no_retweets<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> Result<Response<Vec<i64>>, error::Error>
{
    let mut resp = try!(auth::get(links::users::FRIENDS_NO_RETWEETS, con_token, access_token, None));

    parse_response(&mut resp)
}

///Lookup the relations between the authenticated user and the given accounts.
pub fn relation_lookup(accts: &[UserID], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<RelationLookup>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = accts.iter()
                        .filter_map(|x| match x {
                            &UserID::ID(id) => Some(id.to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(",");
    let name_param = accts.iter()
                          .filter_map(|x| match x {
                              &UserID::ScreenName(name) => Some(name),
                              _ => None,
                          })
                          .collect::<Vec<_>>()
                          .join(",");

    add_param(&mut params, "user_id", id_param);
    add_param(&mut params, "screen_name", name_param);

    let mut resp = try!(auth::get(links::users::FRIENDSHIP_LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Follow the given user with the authenticated account, and set whether device notifications
///should be enabled.
///
///Upon success, this function returns `Ok` with the user that was just followed, even when
///following a protected account. In the latter case, this indicates that the follow request was
///successfully sent.
///
///Calling this with an account the user already follows may return an error, or ("for performance
///reasons") may return success without changing any account settings.
pub fn follow<'a, T: Into<UserID<'a>>>(acct: T, notifications: bool, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    add_param(&mut params, "follow", notifications.to_string());

    let mut resp = try!(auth::post(links::users::FOLLOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Unfollow the given user with the authenticated account.
///
///Upon success, this function returns `Ok` with the user that was just unfollowed.
///
///Calling this with an account the user doesn't follow will return success, even though it doesn't
///change any settings.
pub fn unfollow<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::post(links::users::UNFOLLOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Update notification settings and reweet visibility for the given user.
///
///Calling this for an account the authenticated user does not already follow will not cause them
///to follow that user. It will return an error if you pass `Some(true)` for `notifications` or
///`Some(false)` for `retweets`. Any other combination of arguments will return a `Relationship` as
///if you had called `relation` between the authenticated user and the given user.
pub fn update_follow<'a, T>(acct: T, notifications: Option<bool>, retweets: Option<bool>,
                            con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Relationship>, error::Error>
    where T: Into<UserID<'a>>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    if let Some(notifications) = notifications {
        add_param(&mut params, "device", notifications.to_string());
    }
    if let Some(retweets) = retweets {
        add_param(&mut params, "retweets", retweets.to_string());
    }

    let mut resp = try!(auth::post(links::users::FRIENDSHIP_UPDATE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
