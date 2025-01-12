# Concepts

## StrictYAML

Pullconf uses [StrictYAML](https://hitchdev.com/strictyaml) as configuration format. Pullconf validates resources extensively to ensure that erroneous user input is noticed early and minimize the chance that a client applies syntactically or semantically wrong configuration. 

For example Pullconf ensures that an IP address provided to a [host](configuration/resources/host.md) resource is indeed a valid IP address.

For this reason YAML literals in configuration files might as well all be interpreted as strings before being further scrutinized by **pullconfd** and parsed to a more specific, internal type. Among other things StrictYAML does exactly that.

The YAML parser is only concerned with three types of nodes:

- hashes/maps
- arrays
- string literals

Reducing the YAML feature set in that way has the added side effect of making Pullconf configuration files more readable than other configuration formats would be able to.

## Server component

The Pullconf server (**pullconfd**) is a simple server application that parses resources defined in StrictYAML configuration files and exposes them via an HTTP API. Clients (**pullconf**) use this API to request their designated list of resources via HTTP. All resources remain in memory until the server is restarted or reloaded.

The server also validates configuration extensively and fails if some part of the configuration is erroneous or ambiguous. While not possible in every single case, this approach should prevent any faulty configuration making its way to the client.

When a client requests its resource list it does so using its hostname and an API key. Unless the server is able to tie the provided API key to the list of resources the client is requesting, the request it will be rejected. How a client's hostname and their API key are tied to each other will be further explained in the sections below.

## Client component

Pullconf clients (**pullconf**) request their designated list of resources from **pullconfd** and apply it. Authentication is implemented via API keys and hostnames. When a client's hostname (output from `$ hostname --fqdn`) is `my.example.domain` it fetches its resource list from `HTTP /api/clients/my.example.domain/resources`. A client API key is authorized for this single endpoint and is otherwise rejected to prevent a client from requesting another client's resources.

The resource list is applied in the following way: Every resource contains relationship data. Dependencies are one type of relationship that is common to all resource types. If a resource does not depend on other resources or its dependencies have already been applied, it will be applied in this instant. Otherwise it will be appended to the end of a queue and re-tried after the remaining resources have been applied. The client iterates over the resource queue until every resource has been applied.

## Resources

Resources are the main building blocks for configuring a client system. There are generic types such as [file](configuration/resources/file.md), [directory](configuration/resources/directory.md) or [symlink](configuration/resources/symlink.md). On the other hand there are also very specific resource types as such as [host](configuration/resources/host.md) that serve a single purpose, in this case maintaining a line in `/etc/hosts`.

While generic types could be used to configure most software and components, using specific types (where applicable) enables additional validation that is specific to the domain at hand, e.g. to ensure that entries in `/etc/hosts` are valid and only certain non-default entries are managed.

Pullconf tries to infer dependencies between configured resources. For example every [host](configuration/resources/host.md) resource depends on the target file `/etc/hosts`. If this file does not exist the resource cannot be applied. If an administrator chose to also manage this file explicitly via the [file](configuration/resources/file.md) resource type, the former will depend on the latter and both resources will be applied in order. 

Similarly a [directory](configuration/resources/directory.md) resource depends on other directory resources if those happen to be ancestors to this directory, e.g. `/my/absolute/path` depends on `/my/absolute` if both happen to be managed resources.

## Groups

As previously explained a client's resource list is defined in a StrictYAML file on the server. The file may contain an optional key that can be used to assign groups to a client. Groups are also StrictYAML files stored on the server that (just as client configuration files) include resources. Groups are a place where common resources are defined that apply to multiple clients. For example the sshd configuration of a client fleet could be defined in a group since this might very well be the same on every system.

When the server is (re)started it reads resources from every group that a client is a member of and evaluates them in the context of this client. Resources that are already defined in the client take precedence over those defined in a group when they identify the same resource. For example when a [file](configuration/resources/file.md) resource managing a file at `/my/elaborate/path` is defined in both the client and the group configuration file, the former takes precedence and the latter will be ignored.

While a client's resources take precedence over those defined in groups, there is no hierarchy on the group level. As such there are a total *two* configuration levels, the client level on top and all the group level below it. This structure invites you to compose your client's configuration from a set of groups and leave the client configuration file for the specific stuff, while also avoiding complex inheritance rules that might lead to errors.

