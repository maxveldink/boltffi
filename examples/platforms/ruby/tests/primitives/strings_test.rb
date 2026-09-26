# frozen_string_literal: true

class StringsTest < Minitest::Test
  def test_echo_string
    assert_equal("hello", Demo.echo_string("hello"))
    assert_equal("", Demo.echo_string(""), "case:primitives.strings.string.should_roundtrip_empty")
    assert_equal("café", Demo.echo_string("café"))
    assert_equal("日本語", Demo.echo_string("日本語"))
    assert_equal("hello 🌍 world", Demo.echo_string("hello 🌍 world"), "case:primitives.strings.string.should_roundtrip_emoji")
  end

  def test_concat_strings
    assert_equal("foobar", Demo.concat_strings("foo", "bar"), "case:primitives.strings.string.should_concatenate_values")
    assert_equal("bar", Demo.concat_strings("", "bar"))
    assert_equal("foo", Demo.concat_strings("foo", ""))
    assert_equal("🎉🎊", Demo.concat_strings("🎉", "🎊"))
  end

  def test_string_length
    assert_equal(5, Demo.string_length("hello"))
    assert_equal(0, Demo.string_length(""))
    assert_equal(5, Demo.string_length("café"), "case:primitives.strings.string.should_report_utf8_byte_length")
    assert_equal(4, Demo.string_length("🌍"))
  end

  def test_string_is_empty
    assert_equal(true, Demo.string_is_empty(""), "case:primitives.strings.string.should_detect_empty")
  end

  def test_repeat_string
    assert_equal("ababab", Demo.repeat_string("ab", 3), "case:primitives.strings.string.should_repeat_value")
  end

  def test_borrowed_static_string
    assert_equal("borrowed static", Demo.borrowed_static_string(), "case:primitives.strings.borrowed_static_string.should_return_value")
  end
end
