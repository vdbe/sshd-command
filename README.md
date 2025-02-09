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
{% macro principal(name) -%}
    {{ name }}
    {{ user.name }}@{{ name }}
    {%- for group in user.groups  %}
        {%- if group.gid >= 1000 %}
           @{{- group.name }}@{{ name }}
        {%- endif %}
    {%- endfor -%}
{%- endmacro input -%}

{{- self::principal(name=hostname) }}
{% for search_domain in search_domains  %}
    {{- self::principal(name=hostname ~ "." ~ search_domain) }}
{%  endfor -%}
```

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

## Thanks to
- [catppuccin/whiskers](https://github.com/catppuccin/whiskers) for the inspiration
- [getchoo/nixpkgs-tracker-bot](https://github.com/getchoo/nixpkgs-tracker-bot) for the nix parts
