# Dependencies

Dependencies between resources can be *implicit* or *explicit*.

- *implicit* dependencies between resources are established by **pullconfd** itself according to certain internal rules. For instance, a [directory](resources/directory.md) resource depends implicitly on other directory resources if they happen to be a parent node to this directory within the filesystem.

  There is a section in each resource documentation section that describes the kind of implicit dependency relationships a resource establishes with other resources. Implicit dependencies are established without specific configuration by the user.

- *explicit* dependencies go beyond implicit dependencies in cases where implicit dependencies do not suffice and **pullconfd** cannot infer a relationship between two resources. 

  Explicit dependencies are configured via the `requires` array that is common to all resource types (see [resources](resources/index.md)).

Explicit dependencies are validated with additional care to avoid dependency loops. Explicit dependencies may also produce other errors during validation if a dependency between two resources cannot be established in a logical sense.

For example a [directory](resources/directory.md) resource at `/my/example` cannot depend on another directory resource at `/my/example/further/down`, because the former *must* be processed before the latter.

Dependencies between resources determine the order in which they are applied. They also determine if a resource is applied at all, based on the state of other resources that a resource depends on. In regard to the latter the following rules apply:

* When a resource depends on another resource that failed to apply, the resource is skipped.
* When a resource depends on another resource that was skipped, the resource is skipped as well.
* When a resource's `ensure` parameter is set to `present` or an equivalent value, but it also depends on a resource that is `absent` (or equivalent), the resource fails[^note].

[^note]: There is one exception to this rule: an `execute` resource can still be applied when it depends on an `absent` resource. This enables use cases such as reloading the system manager via (`systemctl daemon-reload`) when a unit [file](resources/file.md) is deleted by setting it to `absent`.
  
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
