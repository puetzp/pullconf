# Introduction

Pullconf is a configuration management system for Debian GNU/Linux and other Debian-based Linux servers. It is heavily influenced by [Puppet](https://puppet.com) (a popular and widely-used configuration management system). In contrast to other configuration management systems this project focuses a lot on simplicity and ease of use. Or to put it in other words: its primary goal is being *boring*. Ideally as boring as its uninspired name.

Pullconf uses a simple client-server architecture: Clients communicate with a central server in order to retrieve a list of resources via an HTTP API. These resources are then applied on the client to achieve a desired state, e.g. create a [file](configuration/resources/file.md) at a certain location.

- the server component is called **pullonfd** and runs as a daemon
- the client component is called **pullconf** and is run at certain intervals to apply resources

As the name implies Pullconf follows a **pull-based** approach to system configuration: a client actively fetches a designated list of resources and applies it according to a certain schedule. Pullconf thereby ensures that every resource maintains its desired state, e.g. that a file has specific content and is owned by a certain user.

**Resources** such as [file](configuration/resources/file.md), [directory](configuration/resources/directory.md) or [user](configuration/resources/user.md) are defined in configuration files on the server. These files follow the [StrictYAML](https://hitchdev.com/strictyaml) syntax.

Scripting and the development of custom modules is *not* supported and out of scope of this project. It focuses instead on including all kinds of resources directly into the project source, thus prioritizing performance and correctness at the cost of flexibility and development speed. However it should ultimately be possible to create simple [file](configuration/resources/file.md)s but also install and manage the configuration of complex software, e.g. a Prometheus server, by defining them as resources.

The following is a basic example for a client configuration file to get a sense of the way StrictYAML is used to define resources:

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
	  # `${pullconf::hostname}` is a pre-defined variable that evaluates to `blechkiste.local`
	  hostname: ${pullconf::hostname}
	  ip_address: ${pullconf::ip_address}

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

## Use cases

As far as configuration management systems go, Pullconf's features are and will remain to be quite minimal and straightforward.

You might want to consider using it when:

- you operate a fleet of homogeneous server systems and your needs for extensive customization are thus low.
- you are just getting started with system configuration management and other, more powerful systems such as Ansible, Puppet or Chef may be overkill.
- all the resource types that you require are covered by Pullconf.
- you do not care so much about DRY ("don't repeat yourself") and value having your client configuration in a comparatively flat structure, instead of having a multi-layered hierarchy and complex rules of inheritance.
- you can live without a DSL (domain-specific language) that would enable you to conditionally include or exclude resources or resource parameters.
- you want to turn your pet systems to cattle (to some extent), because it just happens that this is what configuration management systems are for.

