# group

This resource manages a user group on the client.

## Relationship to other resources

A groups implicitly depends on [user](user.md) resources whose `group` parameter matches the `name` of this group, thereby making this group their primary group. This means that primary groups will be processed after user resources, because in this case the lifecycle of the group is left to the system: A typical Debian installation will create the primary group of a user automatically upon user creation and delete it when the user is deleted.

> It is recommended not to manage the primary groups of [users](user.md) at all, only supplementary groups. Instead leave it to [user](user.md) resources and the operating system to create and delete primary groups when necessary.


## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `name` | string | *Primary parameter*: The group name. | yes | |
| `system` | string | Determines if the group is a system group. One of `true` or `false`. | yes | `false` |

## Examples

```yaml
resources:
  - type: group
    parameters:
      name: mygroup
```

```yaml
resources:
  - type: group
    parameters:
	  ensure: present
      name: mygroup
	  system: true
```

