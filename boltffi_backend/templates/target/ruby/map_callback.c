static int {{ name }}(VALUE {{ key }}, VALUE {{ value }}, VALUE boltffi_argument) {
    boltffi_ruby_writer *writer = (boltffi_ruby_writer *)boltffi_argument;
    {{ write_key }}
    {{ write_value }}
    return ST_CONTINUE;
}
