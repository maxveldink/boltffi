# frozen_string_literal: true

# Ruby-specific rules at the native boundary. These tests have no demo case:
# they pin the conversions every Ruby caller relies on.
class ConversionsTest < Minitest::Test
  def test_integers_reject_non_integers_instead_of_truncating
    assert_raises(TypeError) { Demo.echo_i32(1.5) }
    assert_raises(TypeError) { Demo.echo_i32("1") }
    assert_raises(TypeError) { Demo.echo_i32(nil) }
  end

  def test_integers_reject_values_outside_the_rust_type
    assert_raises(RangeError) { Demo.echo_i8(128) }
    assert_raises(RangeError) { Demo.echo_i8(-129) }
    assert_raises(RangeError) { Demo.echo_i32(2**31) }
    assert_raises(RangeError) { Demo.echo_i64(2**63) }
    assert_equal(-2**63, Demo.echo_i64(-2**63))
  end

  def test_unsigned_integers_reject_negative_values
    assert_raises(RangeError) { Demo.echo_u8(-1) }
    assert_raises(RangeError) { Demo.echo_u32(-1) }
    assert_raises(RangeError) { Demo.echo_u64(-1) }
    assert_raises(RangeError) { Demo.echo_usize(-1) }
    assert_raises(RangeError) { Demo.echo_u64(2**64) }
    assert_equal(2**64 - 1, Demo.echo_u64(2**64 - 1))
  end

  def test_floats_accept_integers_and_reject_other_types
    assert_equal(3.0, Demo.echo_f64(3))
    assert_raises(TypeError) { Demo.echo_f64("3.0") }
    assert_raises(TypeError) { Demo.echo_f64(nil) }
    assert_raises(RangeError) { Demo.echo_f32(1e39) }
    assert_equal(Float::INFINITY, Demo.echo_f32(Float::INFINITY))
  end

  # Ruby warns when an Integer is too large for a Float. A warning hook is Ruby
  # code, and no Ruby code may run while an argument is encoded.
  def test_integers_too_large_for_a_float_raise_without_a_warning
    warnings = []
    verbose = $VERBOSE
    Warning.define_singleton_method(:warn) { |message, **| warnings << message }
    $VERBOSE = true

    assert_raises(RangeError) { Demo.echo_f64(1 << 1100) }
    assert_raises(RangeError) { Demo.echo_vec_f64([1.0, 1 << 20_000]) }
    assert_equal((1 << 1000).to_f, Demo.echo_f64(1 << 1000))
    assert_empty(warnings)
  ensure
    $VERBOSE = verbose
    Warning.singleton_class.remove_method(:warn)
  end

  def test_booleans_are_strict
    assert_raises(TypeError) { Demo.echo_bool(nil) }
    assert_raises(TypeError) { Demo.echo_bool(1) }
  end

  def test_strings_must_be_valid_utf8
    error = assert_raises(ArgumentError) { Demo.echo_string("\xff") }
    assert_match(/invalid byte sequence in UTF-8/, error.message)
    assert_raises(Encoding::CompatibilityError) { Demo.echo_string("caf\u00e9".b) }
    assert_raises(Encoding::CompatibilityError) { Demo.echo_string("caf\u00e9".encode("ISO-8859-1")) }
    assert_raises(TypeError) { Demo.echo_string(:symbol) }
  end

  # Encoding runs no Ruby code: a `to_str` method could change a Hash or an
  # Array after the extension wrote its size.
  def test_strings_are_strict_and_never_call_to_str
    convertible = Object.new
    def convertible.to_str = raise("to_str must not run")
    subclass = Class.new(String)

    assert_raises(TypeError) { Demo.echo_string(convertible) }
    assert_raises(TypeError) { Demo.echo_bytes(convertible) }
    assert_raises(TypeError) { Demo.echo_hash_map({ convertible => [1] }) }
    assert_equal("text", Demo.echo_string(subclass.new("text")))
  end

  def test_ascii_only_strings_pass_in_any_ascii_compatible_encoding
    assert_equal("abc", Demo.echo_string("abc".b))
    assert_equal("abc", Demo.echo_string("abc".encode("US-ASCII")))
  end

  def test_returned_strings_are_utf8
    result = Demo.echo_string("caf\u00e9")

    assert_equal(Encoding::UTF_8, result.encoding)
    assert_predicate(result, :valid_encoding?)
    assert_predicate(Demo.echo_string("plain"), :ascii_only?)
  end

  def test_large_values_cross_in_both_directions
    text = "\u00e9" * 100_000
    numbers = (1..50_000).to_a
    names = Array.new(5_000) { |index| "name-#{index}" }

    assert_equal(text, Demo.echo_string(text))
    assert_equal(numbers, Demo.echo_vec_i32(numbers))
    assert_equal(names, Demo.echo_vec_string(names))
  end

  def test_collections_check_their_container_and_element_types
    assert_raises(TypeError) { Demo.echo_vec_i32(nil) }
    assert_raises(TypeError) { Demo.echo_vec_i32([1, "2"]) }
    assert_raises(TypeError) { Demo.echo_vec_string(["a", 1]) }
    assert_raises(TypeError) { Demo.echo_hash_map([]) }
    assert_raises(TypeError) { Demo.echo_hash_map({ "a" => [1, nil] }) }
    assert_raises(RangeError) { Demo.echo_vec_u32([1, -1]) }
  end

  def test_an_argument_that_rust_rejects_raises_instead_of_returning_a_zero_value
    duplicates = {}.compare_by_identity
    duplicates["key".dup] = [1]
    duplicates["key".dup] = [2]

    error = assert_raises(ArgumentError) { Demo.echo_hash_map(duplicates) }
    assert_match(/rejected an argument.*DuplicateMapKey/, error.message)
    assert_equal({ "key" => [3] }, Demo.echo_hash_map({ "key" => [3] }))
  end

  def test_records_are_frozen_data_values
    point = Demo.make_point(1.0, 2.0)

    assert_kind_of(Data, point)
    assert_predicate(point, :frozen?)
    assert_equal(Demo::Point.new(x: 1.0, y: 2.0), point)
    assert_equal({ x: 1.0, y: 2.0 }, point.to_h)
    assert_raises(TypeError) { Demo.echo_point({ x: 1.0, y: 2.0 }) }
    assert_raises(TypeError) { Demo.echo_point(Demo::Person.new(name: "Ada", age: 36)) }
  end

  def test_record_fields_are_checked_when_they_cross
    assert_raises(TypeError) { Demo.echo_point(Demo::Point.new(x: "1", y: 2.0)) }
    assert_raises(TypeError) { Demo.echo_person(Demo::Person.new(name: nil, age: 1)) }
  end

  def test_record_subclasses_cross_as_their_record
    subclass = Class.new(Demo::Point)

    assert_equal(Demo::Point.new(x: 3.0, y: 4.0), Demo.echo_point(subclass.new(x: 3.0, y: 4.0)))
  end

  def test_functions_use_fixed_arity
    assert_equal(1, Demo.method(:echo_string).arity)
    assert_raises(ArgumentError) { Demo.echo_string }
    assert_raises(ArgumentError) { Demo.echo_string("a", "b") }
  end
end
