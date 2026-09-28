# frozen_string_literal: true

class ComplexOptionsTest < Minitest::Test
  def test_optional_strings
    assert_equal("hello", Demo.echo_optional_string("hello"), "case:options.complex.string.should_roundtrip_some")
    assert_nil(Demo.echo_optional_string(nil), "case:options.complex.string.should_roundtrip_none")
    assert_equal(true, Demo.is_some_string("x"), "case:options.complex.string.should_report_some")
    assert_equal(false, Demo.is_some_string(nil), "case:options.complex.string.should_report_none")
    assert_equal("Name_7", Demo.find_name(7), "case:options.complex.string.should_find_name_for_positive_id")
    assert_nil(Demo.find_name(0), "case:options.complex.string.should_return_none_for_non_positive_id")
  end

  def test_optional_point
    point = Demo::Point.new(x: 1.0, y: 2.0)
    assert_equal(point, Demo.echo_optional_point(point), "case:options.complex.point.should_roundtrip_some")
    assert_nil(Demo.echo_optional_point(nil), "case:options.complex.point.should_roundtrip_none")
    assert_equal(Demo::Point.new(x: 3.0, y: 4.0), Demo.make_some_point(3.0, 4.0), "case:options.complex.point.should_make_some")
    assert_nil(Demo.make_none_point(), "case:options.complex.point.should_make_none")
  end

  def test_optional_vecs
    assert_equal([1, 2, 3], Demo.echo_optional_vec([1, 2, 3]), "case:options.complex.vec.should_roundtrip_some")
    assert_nil(Demo.echo_optional_vec(nil), "case:options.complex.vec.should_roundtrip_none")
    assert_equal([], Demo.echo_optional_vec([]), "case:options.complex.vec.should_roundtrip_empty_some")
    assert_equal(2, Demo.optional_vec_length([9, 8]), "case:options.complex.vec.should_report_length_for_some")
    assert_nil(Demo.optional_vec_length(nil), "case:options.complex.vec.should_return_none_for_absent_length")
    assert_equal([0, 1, 2], Demo.find_numbers(3), "case:options.complex.vec.should_find_numbers_for_positive_count")
    assert_nil(Demo.find_numbers(0), "case:options.complex.vec.should_return_none_for_non_positive_number_count")
    assert_equal(["Name_0", "Name_1"], Demo.find_names(2), "case:options.complex.vec_string.should_find_names_for_positive_count")
    assert_nil(Demo.find_names(0), "case:options.complex.vec_string.should_return_none_for_non_positive_name_count")
  end

  def test_vec_optional_i32
    assert_equal([1, nil, 2, nil, 3], Demo.echo_vec_optional_i32([1, nil, 2, nil, 3]), "case:options.complex.vec_optional_i32.should_roundtrip_mixed_presence")
    assert_equal([], Demo.echo_vec_optional_i32([]), "case:options.complex.vec_optional_i32.should_roundtrip_empty")
    assert_equal([nil, nil, nil], Demo.echo_vec_optional_i32([nil, nil, nil]), "case:options.complex.vec_optional_i32.should_roundtrip_all_none")
  end
end
