{%- if let Some(decoder) = function.decoder %}
static VALUE {{ decoder.name }}(boltffi_ruby_reader *reader) {
    VALUE boltffi_value = Qnil;
    {{ decoder.body }}
    return boltffi_value;
}

{% endif -%}
static VALUE {{ function.wrapper }}({% if function.variadic() %}int argc, VALUE *argv, VALUE self{% else %}VALUE self{% for index in 0..function.arity %}, VALUE boltffi_arg_{{ index }}{% endfor %}{% endif %}) {
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
