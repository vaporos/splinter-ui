// Copyright 2019 Cargill Incorporated
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::time::{Duration, SystemTime};

use actix_web::{error, web, Error, HttpResponse};
use futures::Future;
use gameroom_database::{helpers, models::GameroomNotification, ConnectionPool};

use crate::rest_api::RestApiResponseError;

use super::Paging;

#[derive(Debug, Serialize)]
struct NotificationListResponse {
    data: Vec<ApiNotification>,
    paging: Paging,
}

#[derive(Debug, Serialize)]
struct ApiNotification {
    id: i64,
    notification_type: String,
    org: String,
    target: String,
    timestamp: u64,
    read: bool,
}

impl ApiNotification {
    fn from(db_notification: GameroomNotification) -> ApiNotification {
        ApiNotification {
            id: db_notification.id,
            notification_type: db_notification.notification_type.to_string(),
            org: db_notification.requester.to_string(),
            target: db_notification.target.to_string(),
            timestamp: db_notification
                .created_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::new(0, 0))
                .as_secs(),
            read: db_notification.read,
        }
    }
}

pub fn fetch_notificaiton(
    pool: web::Data<ConnectionPool>,
    notification_id: web::Path<i64>,
) -> Box<dyn Future<Item = HttpResponse, Error = Error>> {
    Box::new(
        web::block(move || get_notification_from_db(pool, *notification_id)).then(
            |res| match res {
                Ok(notification) => Ok(HttpResponse::Ok().json(notification)),
                Err(err) => match err {
                    error::BlockingError::Error(err) => match err {
                        RestApiResponseError::NotFound(err) => {
                            Ok(HttpResponse::NotFound().json(json!({
                                "message": format!("Not Found error: {}", err.to_string())
                            })))
                        }
                        _ => Ok(HttpResponse::BadRequest().json(json!({
                            "message": format!("Bad Request error: {}", err.to_string())
                        }))),
                    },
                    error::BlockingError::Canceled => Ok(HttpResponse::InternalServerError()
                        .json(json!({ "message": "Failed to fetch notification" }))),
                },
            },
        ),
    )
}

fn get_notification_from_db(
    pool: web::Data<ConnectionPool>,
    id: i64,
) -> Result<ApiNotification, RestApiResponseError> {
    if let Some(notification) = helpers::fetch_notification(&*pool.get()?, id)? {
        return Ok(ApiNotification::from(notification));
    }
    Err(RestApiResponseError::NotFound(format!(
        "Notification id: {}",
        id
    )))
}
