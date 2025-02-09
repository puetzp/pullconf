# Triggers

Triggers are closely tied to the [execute](resources/execute.md) resource. Every resource has the ability to trigger an [execute](resources/execute.md) resource through the `triggers` meta-parameter. Just like `requires` this meta-parameter is available to all resources by default.

An [execute](resources/execute.md) resource that is triggered by other resources is only applied when at least one of those resources is applied successfully:

* when the resource is successfully created
* when the resource is successfully deleted
* when the resource is successfully updated

When multiple resources are configured to trigger the same [execute](resources/execute.md) resource, the latter is applied only once, after each of those resources have run to completion.

Since only [execute](resources/execute.md) resources can be triggered, the `trigger` meta-parameter accepts only references to [execute](resources/execute.md) resources.

## Example

```yaml
<...>

resources:
  - type: execute
    parameters:
	  ensure: present
      name: reload-sshd
	  command:
	    - systemctl
	    - reload
		- sshd.service

  - type: file
    parameters:
      ensure: present
      path: /etc/ssh/sshd_config
	  owner: root
	  group: root
	  content:
	    value: |
          Port 22
		  Listen 0.0.0.0
		  
		  PermitRootLogin no
		  PasswordAuthentication no
		  PubkeyAuthentication yes
		  AllowTcpForwarding yes
    triggers:
      - type: execute
        name: reload-sshd

<...>
```
