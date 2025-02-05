# Introduction

Pullconf is a configuration management system for Debian GNU/Linux and other Debian-based Linux servers. It is heavily influenced by [Puppet](https://puppet.com) (a popular and widely-used configuration management system). In contrast to other configuration management systems this project focuses a lot on simplicity and ease of use. Or to put it in other words: its primary goal is being *boring*. Ideally as boring as its uninspired name.

Pullconf uses a simple client-server architecture: Clients communicate with a central server in order to retrieve a list of resources via an HTTP API. These resources are then applied on the client to achieve a desired state, e.g. create a [file](configuration/resources/file.md) at a certain location.

- the server component is called **pullonfd** and runs as a daemon
- the client component is called **pullconf** and is run at certain intervals to apply resources

As the name implies Pullconf follows a **pull-based** approach to system configuration: a client actively fetches a designated list of resources and applies it according to a certain schedule. Pullconf thereby ensures that every resource maintains its desired state, e.g. that a file has specific content and is owned by a certain user.

**Resources** such as [file](configuration/resources/file.md), [directory](configuration/resources/directory.md) or [user](configuration/resources/user.md) are defined in configuration files on the server. These files follow the [StrictYAML](https://hitchdev.com/strictyaml) syntax.

Scripting and the development of custom modules and resources is *not* supported and out of scope of this project. Unlike other configuration management software such as Puppet, Pullconf does *not* provide a DSL (domain-specific language), other than the specific syntax of the StrictYAML configuration and the fixed set of options that the system understands.

Instead Pullconf provides a limited, but versatile set of resources to produce a certain state on the client system.

Most notably the [file](configuration/resources/file.md) resource can be used to install a file on the client, set parts of its content based on commands that run on the client, and run other commands when the file changes, e.g. to restart a service. Since "everything is a file" in Linux, this resource alone does a lot of the heavy lifting.

Pullconf encourages administrators to:

* simply copy their battle-tested configuration (e.g. for services such as `rsyslog`) as-is into a Pullconf configuration file, instead of having to translate the configuration to another format that is specific to the configuration management system at hand.
* use scripts that are local to the client to set file content dynamically, e.g. determine the client's primary IP address and add it as listen address to `/etc/ssh/sshd_config`. Since Pullconf simply takes the script output to replace placeholders, any scripting language can be used.

The following is a basic example for a client configuration file to get a sense of the way StrictYAML is used to define resources:

```yaml
# /etc/pullconfd/conf.d/blechkiste.local.yaml

type: client
name: blechkiste.local
api_key: 20b5094257d70c8d126cf278510b6443d5139e86e18be1389b90a28d526c8236
groups:
  - postfix
  - nginx
  - hardening

variables:
  sshd-port: 22

resources:
  - type: host
    parameters:
      hostname: proxy
      ip_address: 172.16.10.5
      aliases:
        - proxy.local

  - type: apt::package
    parameters:
      ensure: present
      name: openssh-server

  - type: file
    parameters:
      ensure: present
      path: /usr/local/sbin/show-ip-address
      mode: 0755
      owner: root
      group: root
      content:
        value: |
          #!/bin/bash
          ip --json -4 address show dev $1 | jq --raw-output '.[0].addr_info[0].local'

  - type: file
    parameters:
      ensure: present
      path: /etc/ssh/sshd_config
      mode: 0640
      owner: root
      group: root
      content:
        value: |
          Port ${pullconf::sshd-port}
          Listen <ip-address>
        replace:
          - placeholder: <ip-address>
            command:
              - /usr/local/sbin/show-ip-address
              - ens192
    requires:
      - type: apt::package
        name: openssh-server
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
