# Fastly Shadow Traffic

`fastly-shadow-traffic` is a tool to consum data from fastly's syslog service
and perform actions on it.

The main feature for this tool at the moment is the `forwarder`, which replicates
GET and HEAD requests to a desired http server.

## How to use

Within Fastly it is required to [setup](https://docs.fastly.com/guides/streaming-logs/log-streaming-syslog) a `syslog` logging endpoint, in the service of choice.
This logging endpoint will take `GET` and `HEAD` requests and will send them to the URI of
your choice.

To create we need to name the service, this can be anything that we want.

In our case: `load_test`

Then we need to define the log format, we will use the following snippet to send a JSON payload over syslog:

```json
{ "time":%{time.start.sec}V, "event": { "service_id":"%{req.service_id}V", "client_ip":"%h", "request":"%m", "url":"%{cstr_escape(req.url)}V", "request_referer":"%{Referer}i", "request_user_agent":"%{User-Agent}i", "request_accept_content":"%{Accept}i", "request_accept_language":"%{Accept-Language}i", "request_accept_encoding":"%{Accept-Encoding}i", "request_accept_charset":"%{Accept-Charset}i" } }
```

We then need to specify a Log line address, this will be the URI defined in this applications configuration. `voguede-load-test.staging.cni.digital:1513`

And finally under Advanced configuration the log line format needs to be set as blank.

Once this application is deployed on the right port, we should start to see traffic being recieved by `fastly-shadow-traffic`

To ensure that this application targets the correct host, we can use the following commandline arguments to point it at a certain host, through a certain ingress:

```
args:
  - --target
  - http://vogue-de-rocket.staging.cni.digital
  - --host
  - traefik-restricted-ingress-experience
  - --level
  - warning
```

Finally, your application should start to recieve http requests by the forwarder.

### Doc

[HERE](./doc)

### Developing

To build for release use the `docker-image` target in the make file, it is also
wise to update the documentation as part of the release with the target `doc`

### Installing dependencies

```
cargo build
```

```
cargo doc
```

#### Linting

```
```

#### Testing

```
cargo test
```

