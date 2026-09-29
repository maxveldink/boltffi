static VALUE {{ function.wrapper }}({% if function.variadic() %}int argc, VALUE *argv, VALUE self{% else %}VALUE self{% for index in 0..function.arity %}, VALUE boltffi_arg_{{ index }}{% endfor %}{% endif %}) {
{%- if function.variadic() %}
    rb_check_arity(argc, {{ function.arity }}, {{ function.arity }});
{%- endif %}
    (void)self;
{%- for statement in function.setup %}
    {{ statement }}
{%- endfor %}
    {{ function.call }}
    return {{ function.result }};
}
