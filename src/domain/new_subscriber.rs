use crate::domain::{subscriber_email::SubscriberEmail, subscriber_name::SubscriberName};

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
