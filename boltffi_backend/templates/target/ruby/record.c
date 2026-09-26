static VALUE {{ record.symbols.builder() }}(const VALUE *values) {
    VALUE boltffi_record = rb_obj_alloc({{ record.symbols.class() }});
{%- if record.members.is_empty() %}
    (void)values;
{%- endif %}
{%- for member in record.members %}
    RSTRUCT_SET(boltffi_record, {{ loop.index0 }}, values[{{ loop.index0 }}]);
{%- endfor %}
    return rb_obj_freeze(boltffi_record);
}
{%- match record.body %}
{%- when Body::Direct(direct) %}

BOLTFFI_RUBY_MAYBE_UNUSED static VALUE {{ record.symbols.boxer() }}({{ direct.c_type }} value) {
    VALUE boltffi_fields[{{ record.value_slots() }}] = { Qnil };
{%- if direct.fields.is_empty() %}
    (void)value;
{%- endif %}
{%- for field in direct.fields %}
    boltffi_fields[{{ loop.index0 }}] = boltffi_ruby_from_{{ field.stem }}(value.{{ field.c_name }});
{%- endfor %}
    return {{ record.symbols.builder() }}(boltffi_fields);
}

BOLTFFI_RUBY_MAYBE_UNUSED static {{ direct.c_type }} {{ record.symbols.unboxer() }}(VALUE value) {
    {{ direct.c_type }} record;
    memset(&record, 0, sizeof(record));
    boltffi_ruby_expect_record(value, {{ record.symbols.class() }});
{%- for field in direct.fields %}
    record.{{ field.c_name }} = boltffi_ruby_to_{{ field.stem }}(RSTRUCT_GET(value, {{ loop.index0 }}));
{%- endfor %}
    return record;
}

BOLTFFI_RUBY_MAYBE_UNUSED static VALUE {{ record.symbols.reader() }}(boltffi_ruby_reader *reader) {
    {{ direct.c_type }} value;
    memcpy(&value, boltffi_ruby_read_bytes(reader, sizeof(value)), sizeof(value));
    return {{ record.symbols.boxer() }}(value);
}

BOLTFFI_RUBY_MAYBE_UNUSED static void {{ record.symbols.writer() }}(boltffi_ruby_writer *writer, VALUE value) {
    {{ direct.c_type }} record = {{ record.symbols.unboxer() }}(value);
    boltffi_ruby_write_raw(writer, &record, sizeof(record));
}
{%- when Body::Encoded(encoded) %}
{%- for helper in encoded.helpers %}

{{ helper }}
{%- endfor %}

BOLTFFI_RUBY_MAYBE_UNUSED static VALUE {{ record.symbols.reader() }}(boltffi_ruby_reader *reader) {
    VALUE boltffi_fields[{{ record.value_slots() }}] = { Qnil };
{%- if encoded.fields.is_empty() %}
    (void)reader;
{%- endif %}
{%- for field in encoded.fields %}
    {
        {{ field.read }}
    }
{%- endfor %}
    return {{ record.symbols.builder() }}(boltffi_fields);
}

BOLTFFI_RUBY_MAYBE_UNUSED static void {{ record.symbols.writer() }}(boltffi_ruby_writer *writer, VALUE value) {
    boltffi_ruby_expect_record(value, {{ record.symbols.class() }});
{%- if encoded.fields.is_empty() %}
    (void)writer;
{%- endif %}
{%- for field in encoded.fields %}
    {
        VALUE boltffi_field_{{ loop.index0 }} = RSTRUCT_GET(value, {{ loop.index0 }});
        {{ field.write }}
    }
{%- endfor %}
}
{%- endmatch %}
