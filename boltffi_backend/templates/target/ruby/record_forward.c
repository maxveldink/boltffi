static VALUE {{ record.symbols.class() }} = Qnil;
BOLTFFI_RUBY_MAYBE_UNUSED static VALUE {{ record.symbols.reader() }}(boltffi_ruby_reader *reader);
BOLTFFI_RUBY_MAYBE_UNUSED static void {{ record.symbols.writer() }}(boltffi_ruby_writer *writer, VALUE value);
{%- if let Body::Direct(direct) = record.body %}
BOLTFFI_RUBY_MAYBE_UNUSED static VALUE {{ record.symbols.boxer() }}({{ direct.c_type }} value);
BOLTFFI_RUBY_MAYBE_UNUSED static {{ direct.c_type }} {{ record.symbols.unboxer() }}(VALUE value);
{%- endif %}
