# sshd-command

Sshd-command is tool to be used a `AuthorizedKeysCommand`/`AuthorizedPrincipalsCommand` (see SSHD_CONFIG(5))
in combination with a [tera template](https://keats.github.io/tera/).


## Usage

```
# /etc/ssh/sshd_config
# ...

AuthorizedPrincipalsCommand /usr/local/bin/sshd-command /etc/ssh/templates/principals.tera %U %u
AuthorizedPrincipalsCommandUser nobody
````
```tera
---
# /etc/ssh/templates/principals.tera
sshd_command:
    version: 0.1.0
    command: principals
    tokens: '%U %u'
    hostname: true
    complete_user: true
search_domains:
    - home.arpa
    - local
---
{% macro principals(fqdn) -%}
    {{ fqdn }}
    {{ user.name }}@{{ fqdn }}
    {%- for group in user.groups  %}
        {%- if group.gid >= 1000 %}
           @{{- group.name }}@{{ fqdn }}
        {%- endif %}
    {%- endfor -%}
{%- endmacro input -%}

{{- self::principals(fqdn=hostname) }}
{% for search_domain in search_domains  %}
    {{- self::principals(fqdn=hostname ~ "." ~ search_domain) }}
{%  endfor -%}
```

You can validate the front matter with `sshd-command --validate <template>`.

<details open>
<summary>Output: user@server01: @admin</summary>
    
```
server01
    user@server01
           @admin@server01
server01.home.arpa
    user@server01.home.arpa
           @admin@server01.home.arpa
server01.local
    user@server01.local
           @admin@server01.local
```
</details>

## Installation

Donwload the correct binary from the [latest release](https://github.com/vdbe/sshd-command/releases/tag/v0.2.0).

### Nixos

This project is packaged (and updated/cached) in [vdbe/flake-pkgs](https://github.com/vdbe/flake-pkgs).

package: `legacyPackage.${system}.sshd-command`
<details close>
<summary>nixosModule `nixosModules.sshd-command`</summary>
  
```nix
imports = [
  inputs.mypkgs.nixosModules.sshd-command
];

services.openssh = {
  extraConfig = ''
    TrustedUserCAKeys /etc/ssh/trusted_user_ca
    AuthorizedPrincipalsCommandUser nobody
  '';

  sshd-command = {
    enable = true;
    package = inpust'.mypkgs.sshd-command;
    templates = {
      principals = {
        sshd-command = {
          command = "principals";
          tokens = [
            "%U"
            "%u"
          ];
        };
        extraFrontMatter = {
          search_domains = ["home.arpa" "local"];
        };
        tera = ''
          {% macro principals(fqdn) -%}
          {{ fqdn }}
          {{ user.name }}@{{ fqdn }}
              {%- for group in user.groups  %}
                  {%- if group.gid >= 1000 %}
          @{{- group.name }}@{{ fqdn }}
                  {%- endif %}
              {%- endfor -%}
          {%- endmacro principals -%}

          {{- self::principals(fqdn=hostname) }}
          {% for search_domain in search_domains  %}
          {{- self::principals(fqdn=hostname ~ "." ~ search_domain) }}
          {%  endfor -%}
        '';
      };
    };
  };
};
```
</details>


## Documentation

### Front matter

Front matter options outside of the `sshd_command` scope are added to the terraform context,
all options documented below are in the `sshd_command` scope.

- version (REQUIRED)
  Minimum version required for the template.
- command (REQUIRED)
  For what sshd command is the template: `principals`/`keys`.
- Tokens (REQUIRED)
  Space seperated list of token provided to the command.
  If more then 1 this must be quoted.
- hostname (OPTIONAL)
  Add the systems hostname to the context
- complete_user (OPTIONAL)
  Completes user information from %U or %u (atleast 1 must be provided) with:
  - user id (`user.uid`)
  - user name (`user.name`)
  - primary group id (`user.gid`)
  - user groups (`user.groups[]`)
    - group id (`user.groups[].name`)
    - group name (`user.groups[].gid`)


### Tokens/context

| Token | Context           | Frontmatter                             |
| ----- | ----------------- | --------------------------------------- |
| `%C`  | `client`/`server` | -                                       |
| `%D`  | TODO              | -                                       |
| `%F`  | TODO              | -                                       |
| `%f`  | TODO              | -                                       |
| `%h`  | `home_dir`        | -                                       |
| `%i`  | `key_id`          | -                                       |
| `%K`  | TODO              | -                                       |
| `%k`  | TODO              | -                                       |
| `%S`  | TODO              | -                                       |
| `%T`  | TODO              | -                                       |
| `%t`  | TODO              | -                                       |
| `%U`  | `user.uid`        | `sshd_command.complete_user` (OPTIONAL) |
| `%u`  | `user.name`       | `sshd_command.complete_user` (OPTIONAL) |
| -     | `hostname`        | `sshd_command.hostname`                 |


## Thanks to
- [catppuccin/whiskers](https://github.com/catppuccin/whiskers) for the inspiration
- [getchoo/nixpkgs-tracker-bot](https://github.com/getchoo/nixpkgs-tracker-bot) for the nix parts
