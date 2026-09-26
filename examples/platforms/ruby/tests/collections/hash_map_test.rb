# frozen_string_literal: true

class HashMapTest < Minitest::Test
  def test_make_hash_map
    map = Demo.make_hash_map
    assert_kind_of(Hash, map)
    assert_equal(10, map["first"], "case:collections.hash_map.should_return_values")
    assert_equal(20, map["second"])
  end

  def test_echo_hash_map_empty
    assert_equal({}, Demo.echo_hash_map({}), "case:collections.hash_map.should_roundtrip_empty")
  end

  def test_echo_hash_map_nested_values
    input = { "a" => [1, 2], "b" => [] }
    result = Demo.echo_hash_map(input)
    assert_equal([1, 2], result["a"], "case:collections.hash_map.should_roundtrip_nested_values")
    assert_equal([], result["b"])
  end
end
