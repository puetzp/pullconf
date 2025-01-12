# Pullconf

Pullconf is a **configuration management system** for Debian GNU/Linux and other Debian-based Linux servers. It is heavily influenced by [Puppet](https://puppet.com) (a popular and widely-used configuration management system). In contrast to other configuration management systems this project focuses a lot on simplicity and ease of use. Or to put it in other words: its primary goal is being *boring*. Ideally as boring as its uninspired name.

Pullconf uses a simple client-server architecture: Clients communicate with a central server in order to retrieve a list of resources via an HTTP API. These resources are then applied on the client to achieve a desired state, e.g. create a file at a certain location.

As the name implies Pullconf follows a **pull-based** approach to system configuration: a client actively fetches a designated list of resources and applies it according to a certain schedule. Pullconf thereby ensures that every resource maintains its desired state, e.g. that a file has specific content and is owned by a certain user.

**Resources** such as a file, directory or user are defined in configuration files on the server. These files follow the [StrictYAML](https://hitchdev.com/strictyaml) syntax.

Pullconf does not try to do anything revolutionary in the space of configuration management systems. In fact it tries to be as boring and straightforward as possible. It might fit your needs when:

- you operate a fleet of homogeneous server systems and your needs for extensive customization are thus low.
- you are just getting started with system configuration management and other, more powerful systems such as Ansible, Puppet or Chef may be overkill.
- all the resource types that you require are covered by Pullconf.
- you do not care so much about DRY ("don't repeat yourself") and value having your client configuration in a comparatively flat structure, instead of having a multi-layered hierarchy and complex rules of inheritance.
- you can live without a DSL (domain-specific language) that would enable you to conditionally include or exclude resources or resource parameters.
- you want to turn your pet systems to cattle (to some extent), because it just happens that this is what configuration management systems are for.

More detailed information, installation instructions and configuration examples can be found at [pullconf.dev](https://pullconf.dev).

## Architecture

This section could use a nice drawing, but the architecture can also be explained in a few sentences:

A fleet of clients (Debian-based Linux servers) connects regularly with a central server to fetch their respective configuration/resource catalog. The configuration is stored on the server in a flat directory structure containing TOML files. Clients use their fully-qualified domain name/hostname and an API key to authenticate to the server.

## Features

As already mentioned the resources belonging to a client are compiled from StrictYAML files that follow a certain syntax. There are some features that allow you to manage your configuration effectively:

- Resources can be collected into groups. A client can be a member of any number of groups. Clients inherit the resources defined in groups in addition to their own set of resources. The server prevents you from submitting ambiguous or invalid configuration and provides detailed error messages for conflict resolution. Meanwhile clients continue to be served with the most recent, valid configuration.
- Variables can be defined per client and used throughout configuration files to substitute resource parameters.
- Dependencies between resources are infered to some extent and applied in logical order. In addition explicit dependencies can be defined using the `requires` meta-parameter.

## Documentation

... can be found at [pullconf.dev](https://pullconf.dev)!

## Example

This is a basic example for a client configuration file to get a sense of the way StrictYAML is used to define resources:

```yaml
# /etc/pullconfd/conf.d/blechkiste.local.yaml

type: client
name: blechkiste.local
api_key: 20b5094257d70c8d126cf278510b6443d5139e86e18be1389b90a28d526c8236
groups:
  - sshd
  - postfix
  - nginx
  - hardening

variables:
  ip_address: 172.16.5.6

resources:
  - type: host
    parameters:
	  # `$pullconf::hostname` is a pre-defined variable that evaluates to `blechkiste.local`
	  hostname: $pullconf::hostname
	  ip_address: $pullconf::ip_address

  - type: host
    parameters:
	  hostname: proxy
	  ip_address: 172.16.10.5
	  aliases:
	    - proxy.local

  - type: file
    parameters:
	  path: /etc/logrotate.d/rsyslog
	  owner: root
	  group: root
	  mode: 0644
	  content: |
	    /var/log/syslog
		/var/log/mail.info
		/var/log/mail.warn
		/var/log/mail.err
		/var/log/mail.log
		/var/log/daemon.log
		/var/log/kern.log
		/var/log/auth.log
		/var/log/user.log
		/var/log/lpr.log
		/var/log/cron.log
		/var/log/debug
		/var/log/messages
		{
			rotate 4
			weekly
			missingok
			notifempty
			compress
			delaycompress
			sharedscripts
			postrotate
			/usr/lib/rsyslog/rsyslog-rotate
			endscript
		}
```

## Future development

The basic building blocks for Pullconf are complete. In the future development will focus on adding more and more resource types to complement the very limited set of resources available at this time.
