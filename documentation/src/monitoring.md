# Monitoring

## Server

The **pullconfd** application will fail whenever it fails to validate its configuration. This can happen after a restart or a reload. The application will shutdown gracefully, but return a non-zero exit code, which causes systemd to mark `pullconfd.service` as `failed`.

At this point the only way to detect configuration issues is to monitor the systemd unit or check if a `pullconfd` process is running.

## Client

The **pullconf** application will fail whenever it fails to validate its own configuration (e.g. environment variables) or it fails to apply at least one of its resources.

At this point a sure way to detect when **pullconf** has issues is to monitor the systemd service unit. Since **pullconf** also emits a JSON log, an automatic log processing system might be able to point out issues based on the JSON log output (see [logging](logging.md) for more information on the format). 
