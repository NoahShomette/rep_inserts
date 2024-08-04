use bevy::{color::palettes, prelude::*};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_replicon::{
    core::{replication_rules::AppRuleExt, Replicated},
    prelude::{ParentSync, RepliconChannels},
    RepliconPlugins,
};
use bevy_replicon_renet::{
    client::RepliconRenetClientPlugin,
    renet::{
        transport::{ClientAuthentication, NetcodeClientTransport},
        ConnectionConfig, RenetClient,
    },
    server::RepliconRenetServerPlugin,
};
use bevy_replicon_renet::{
    renet::{
        transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
        RenetServer,
    },
    RenetChannelsExt,
};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RepliconPlugins,
            RepliconRenetClientPlugin,
            RepliconRenetServerPlugin,
            WorldInspectorPlugin::new(),
        ))
        .replicate::<Marker>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                add_children_client,
                apply_deferred,
                update_ui_text_and_count,
            ),
        )
        .run();
}

const PROTOCOL_ID: u64 = 0;

#[derive(Component, Serialize, Deserialize)]
struct Marker;

#[derive(Component, Serialize, Deserialize)]
struct ChildMarker;

fn setup(mut commands: Commands, channels: Res<RepliconChannels>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(TextBundle::from_section(
        format!("Added"),
        TextStyle {
            font_size: 30.0,
            color: palettes::css::RED.into(),
            ..default()
        },
    ));

    if !connect_server(&mut commands, &channels) {
        connect_client(&mut commands, &channels);
        println!("Started Client");
        return;
    }

    println!("Started Server");
}

fn connect_server(commands: &mut Commands, channels: &Res<RepliconChannels>) -> bool {
    let port = 4000;
    let server_channels_config = channels.get_server_configs();
    let client_channels_config = channels.get_client_configs();

    let server = RenetServer::new(ConnectionConfig {
        server_channels_config,
        client_channels_config,
        ..Default::default()
    });

    let Ok(current_time) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) else {
        return false;
    };
    let public_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let Ok(socket) = UdpSocket::bind(public_addr) else {
        return false;
    };
    let server_config = ServerConfig {
        current_time,
        max_clients: 10,
        protocol_id: PROTOCOL_ID,
        authentication: ServerAuthentication::Unsecure,
        public_addresses: vec![public_addr],
    };
    let Ok(transport) = NetcodeServerTransport::new(server_config, socket) else {
        return false;
    };

    commands.insert_resource(server);
    commands.insert_resource(transport);
    commands.spawn((Marker, Replicated));
    true
}

fn connect_client(commands: &mut Commands, channels: &Res<RepliconChannels>) -> bool {
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port = 4000;
    let server_channels_config = channels.get_server_configs();
    let client_channels_config = channels.get_client_configs();

    let client = RenetClient::new(ConnectionConfig {
        server_channels_config,
        client_channels_config,
        ..Default::default()
    });

    let Ok(current_time) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) else {
        return false;
    };
    let client_id = current_time.as_millis() as u64;
    let server_addr = SocketAddr::new(ip, port);
    let Ok(socket) = UdpSocket::bind((ip, 0)) else {
        return false;
    };
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: None,
    };
    let Ok(transport) = NetcodeClientTransport::new(current_time, authentication, socket) else {
        return false;
    };

    commands.insert_resource(client);
    commands.insert_resource(transport);
    true
}

fn add_children_client(
    mut counter: Query<(Entity, &Marker), Without<Children>>,
    mut commands: Commands,
) {
    for (entity, _) in counter.iter() {
        commands.entity(entity).with_children(|builder| {
            builder.spawn(ChildMarker);
        });
    }
}

fn update_ui_text_and_count(
    mut counter: Query<(&Marker, &Children), Added<Marker>>,
    mut text: Query<&mut Text>,
) {
    let Ok(mut counter) = counter.get_single_mut() else {
        println!("Counter not changed");

        return;
    };

    let Ok(mut text) = text.get_single_mut() else {
        return;
    };

    println!("changing text");
    text.sections[0].style.color = palettes::css::GREEN.into();
}
