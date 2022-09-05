//! This example assumes you have an ActiveMQ instant that supports AMQP 1.0
//! running on your localhost
//! 
//! `ActiveMQ` uses alternative TLS establishment (ie. establish TLS without 
//! exchanging ['A', 'M', 'Q', 'P', '2', '1', '0', '0'] header). The user should
//! follow the alternative TLS establishment example which is also copied below.
//! 
//! Please note that you may need to explicitly set you `ActiveMQ` to use TLSv1.2 or higher
//! in the xml configuration file.
//! 
//! ```xml
//! <transportConnector name="amqp+ssl" uri="amqp+ssl://0.0.0.0:5671?transport.enabledProtocols=TLSv1.2"/>
//! ```

use fe2o3_amqp::sasl_profile::SaslProfile;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Receiver;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let addr = "localhost:5671";
    let domain = "localhost";
    let stream = TcpStream::connect(addr).await.unwrap();
    let connector = native_tls::TlsConnector::builder()
        // .danger_accept_invalid_certs(true) // FIXME: uncomment this if you just need a quick test with a self-signed cert
        .build()
        .unwrap();
    let connector = tokio_native_tls::TlsConnector::from(connector);
    let tls_stream = connector.connect(domain, stream).await.unwrap();

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .sasl_profile(SaslProfile::Plain {
            username: "guest".into(),
            password: "guest".into(),
        })
        .open_with_stream(tls_stream)
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();
    let mut receiver = Receiver::attach(&mut session, "rust-receiver-link-1", "q1")
        .await
        .unwrap();

    let message = "hello AMQP";
    let outcome = sender.send(message).await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();
    println!("Sent: {:?}", message);

    let delivery = receiver.recv::<String>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    println!("Received: {:?}", delivery.try_as_value().unwrap());

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}