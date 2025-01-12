# user

This resource manages a user on the client.

## Relationship to other resources

A user implicitly depends on [group](group.md) resources whose `name` parameter matches one of the strings in the `groups` array as supplementary groups must be processed first.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `name` | string | *Primary parameter*: The user name. | yes | |
| `system` | string | Determines if the user is a system user. One of `true` or `false`. | yes | `false` |
| `comment` | string | An optional comment to add to the database entry for this user. | no | |
| `shell` | string | The path to the login shell of this user. *Note that when this parameter is omitted, platform-dependent defaults may apply.* | no | |
| `home` | string | The home directory of this user. | no | `/home/<name>` |
| `password` | string | The password hash of this user. | no | `!` which means "locked" |
| `expiry_date` | string | The date at which the account should expire, in the format `YYYY-MM-DD`. *Note that when this parameter is omitted, platform-dependent defaults may apply.* | no | |
| `group` | string | The name of the user's primary group. | no | same as `name` |
| `groups` | array | A list of names of supplementary groups that the user should be a member of. Array items must be strings. | no | |

## Examples

```yaml
resources:
  - type: user
    parameters:
      name: myuser
```

```yaml
resources:
  - type: user
    parameters:
	  ensure: present
      name: myuser
	  system: true
      comment: Employee of the Year
      shell: /bin/zsh
      home: /home/myuser
      password: $6$ugth6io4j7fQHBxh$oDr51KYqju5jMr/lCsYpAouzxOINxhyZhiSRH1220TOZ8VRMxxNaGXnv.JzH/XUN9oezau7sKqrBlcdQfqmGv0
      expiry_date: 2025-12-31
      group: myuser
      groups:
	    - ssh-login
	    - team-xyz
	  
```

