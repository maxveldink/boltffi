{%- for helper in function.helpers %}
{{ helper }}

{% endfor -%}
{%- if let Some(decoder) = function.decoder %}
static VALUE {{ decoder.name }}(boltffi_ruby_reader *reader) {
    VALUE boltffi_value = Qnil;
    {{ decoder.body }}
    return boltffi_value;
}

{% endif -%}
static VALUE {{ function.wrapper }}({{ function.signature() }}) {
{%- if function.variadic() %}
    rb_check_arity(argc, {{ function.arity }}, {{ function.arity }});
{%- endif %}
    (void)self;
{%- for statement in function.setup %}
    {{ statement }}
{%- endfor %}
    {{ function.call }}
{%- for statement in function.cleanup %}
    {{ statement }}
{%- endfor %}
{%- if let Some(check) = function.check %}
    {{ check }}
{%- endif %}
    return {{ function.result }};
}
