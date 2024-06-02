use dbus_async::DBus;
use dbus_message_parser::message::Message;
use dbus_message_parser::value::Value;
use std::{collections::HashMap, convert::TryInto};

use serde::{Serialize, Deserialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
struct DBusMessage {
    name: String,
    icon: String,
    state: String,
    action: String
}

pub async fn send_signal(signature_key_string: &str, button_name: &str, key_or_button: &str, action: &str, button_state: &str) {
    let (dbus, _connection_handle) = DBus::session(true, true)
        .await
        .expect("failed to get the DBus object");
   
    let mut msg = Message::signal(
        "/org/gnome/shell/extensions/elgatopedalcompanion".try_into().unwrap(),
        "org.gnome.shell.extensions.Elgatopedalcompanion".try_into().unwrap(),
        "PedalActionSignal".try_into().unwrap(),
    );

    let button_message = DBusMessage {
        name: button_name.to_string(),
        icon: "media-playback-start".to_string(),
        state: button_state.to_string(),
        action: key_or_button.to_string(),
    };

    let notification_value = json!([
        [
            button_name,
            button_message
        ]
    ]);

    let mut notification = HashMap::new();
    notification.insert(signature_key_string, notification_value);

    let json_message = serde_json::to_string_pretty(&notification).unwrap();
    
    msg.add_value(Value::String(json_message));
    
    
    let _ = dbus.send(msg);
}