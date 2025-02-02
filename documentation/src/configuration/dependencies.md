# Dependencies

Dependencies between resources can be *implicit* or *explicit*.

- *implicit* dependencies between resources are established by **pullconfd** itself according to certain internal rules. For instance, a [directory](resources/directory.md) resource depends implicitly on other directory resources if they happen to be a parent node to this directory within the filesystem.

  There is a section in each resource documentation section that describes the kind of implicit dependency relationships a resource establishes with other resources. Implicit dependencies are established without specific configuration by the user.

- *explicit* dependencies go beyond implicit dependencies in cases where implicit dependencies do not suffice and **pullconfd** cannot infer a relationship between two resources. 

  Explicit dependencies are configured via the `requires` array that is common to all resource types (see [resources](resources/index.md)).

Explicit dependencies are validated with additional care to avoid dependency loops. Explicit dependencies may also produce other errors during validation if a dependency between two resources cannot be established in a logical sense.

For example a [directory](resources/directory.md) resource at `/my/example` cannot depend on another directory resource at `/my/example/further/down`, because the former *must* be processed before the latter.

When one resource requires another, it is ensured that the required resource is applied first. However should the required resource fail to apply (e.g. because a file cannot be written due to missing permissions), other resources that depend on this resource via `requires` will be skipped.

Within the `requires` array other resources are usually referred to by their `type` (e.g. `file`) and their *primary parameter*. For example a [directory](resources/directory.md) is primarily identified by its `path` parameter.

The primary parameter of a resource is marked in each resource's documentation section.

## Example

This example resource contains multiple explicit dependencies.

```yaml
<...>

resources:
  - type: directory
    parameters:
	  path: /my/simple/example
    requires:
	  - type: file
	    path: /totally/different/location
	  - type: host
	    ip_address: 127.0.0.1

<...>
```
