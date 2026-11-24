//! BlueZ state and change notifications over the system D-Bus.

use std::future::poll_fn;
use std::pin::Pin;
use std::sync::mpsc::Sender;
use std::task::Poll;

use zbus::export::futures_core::Stream;
use zbus::fdo::ObjectManagerProxy;
use zbus::message::Type;
use zbus::{Connection, MatchRule, MessageStream};

use crate::Result;

const BLUEZ: &str = "org.bluez";
const DEVICE_INTERFACE: &str = "org.bluez.Device1";

pub fn connect() -> Result<Connection> {
    Ok(zbus::block_on(Connection::system())?)
}

/// Whether a connected Bluetooth device has a name starting with `prefix`.
pub fn is_connected(connection: &Connection, prefix: &str) -> Result<bool> {
    let objects = zbus::block_on(async {
        ObjectManagerProxy::builder(connection)
            .destination(BLUEZ)
            .path("/")
            .build()
            .await?
            .get_managed_objects()
            .await
    });
    let objects = match objects {
        Ok(objects) => objects,
        // bluetoothd is not running, so nothing is connected.
        Err(zbus::fdo::Error::ServiceUnknown(_) | zbus::fdo::Error::NameHasNoOwner(_)) => {
            return Ok(false);
        }
        Err(error) => return Err(error.into()),
    };

    Ok(objects.values().any(|interfaces| {
        let Some(device) = interfaces.get(DEVICE_INTERFACE) else {
            return false;
        };
        let connected = device
            .get("Connected")
            .and_then(|value| bool::try_from(value).ok())
            .unwrap_or(false);
        let name = ["Name", "Alias"]
            .iter()
            .find_map(|key| <&str>::try_from(device.get(*key)?).ok())
            .unwrap_or("");
        connected && name.starts_with(prefix)
    }))
}

/// Send an event once subscribed, then one for every change that may affect
/// which devices are connected. Returns only when the bus goes away.
pub fn watch(events: Sender<()>) -> Result<()> {
    zbus::block_on(async {
        let connection = Connection::system().await?;

        // Property changes on adapters and devices.
        let properties = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .member("PropertiesChanged")
            .path_namespace("/org/bluez")
            .build()?;
        // bluetoothd starting or stopping.
        let owner = MatchRule::builder()
            .msg_type(Type::Signal)
            .sender("org.freedesktop.DBus")
            .interface("org.freedesktop.DBus")
            .member("NameOwnerChanged")
            .arg(0, BLUEZ)
            .build()?;
        let mut properties = MessageStream::for_match_rule(properties, &connection, None).await?;
        let mut owner = MessageStream::for_match_rule(owner, &connection, None).await?;

        loop {
            if events.send(()).is_err() {
                return Ok(());
            }
            let message = poll_fn(|context| {
                for stream in [&mut properties, &mut owner] {
                    if let Poll::Ready(message) = Pin::new(stream).poll_next(context) {
                        return Poll::Ready(message);
                    }
                }
                Poll::Pending
            })
            .await;
            match message {
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(error.into()),
                None => return Ok(()),
            }
        }
    })
}
