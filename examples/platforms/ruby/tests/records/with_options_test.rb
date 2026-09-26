# frozen_string_literal: true

class OptionalFieldRecordTest < Minitest::Test
  def test_user_profile
    user_profile = Demo.make_user_profile("Alice", 30, "alice@example.com", 98.5)
    assert_equal(Demo::UserProfile.new(name: "Alice", age: 30, email: "alice@example.com", score: 98.5), user_profile, "case:records.with_options.user_profile.should_make_with_present_options")
    assert_equal(user_profile, Demo.echo_user_profile(user_profile), "case:records.with_options.user_profile.should_roundtrip_present_options")
    assert_equal("Alice <alice@example.com>", Demo.user_display_name(user_profile), "case:records.with_options.user_profile.should_display_email_when_present")

    user_without_email = Demo.make_user_profile("Bob", 22, nil, nil)
    assert_equal(Demo::UserProfile.new(name: "Bob", age: 22, email: nil, score: nil), user_without_email, "case:records.with_options.user_profile.should_make_with_absent_options")
    assert_equal(user_without_email, Demo.echo_user_profile(user_without_email), "case:records.with_options.user_profile.should_roundtrip_absent_options")
    assert_equal("Bob", Demo.user_display_name(user_without_email), "case:records.with_options.user_profile.should_display_name_when_email_absent")

    user_mixed_options = Demo.make_user_profile("Cleo", 27, "cleo@example.com", nil)
    assert_equal(user_mixed_options, Demo.echo_user_profile(user_mixed_options), "case:records.with_options.user_profile.should_roundtrip_mixed_options")
    user_utf8 = Demo.make_user_profile("Élodie", 31, "élodie@café.example", 88.25)
    assert_equal(user_utf8, Demo.echo_user_profile(user_utf8), "case:records.with_options.user_profile.should_roundtrip_utf8_optional_string")
  end

  def test_search_result
    search_result = Demo::SearchResult.new(query: "rust ffi", total: 12, next_cursor: "cursor-1", max_score: 0.99)
    assert_equal(search_result, Demo.echo_search_result(search_result), "case:records.with_options.search_result.should_roundtrip_present_options")
    search_result_absent = Demo::SearchResult.new(query: "rust ffi", total: 0, next_cursor: nil, max_score: nil)
    assert_equal(search_result_absent, Demo.echo_search_result(search_result_absent), "case:records.with_options.search_result.should_roundtrip_absent_options")
    assert_equal(true, Demo.has_more_results(search_result), "case:records.with_options.search_result.should_report_more_results_when_cursor_present")
    assert_equal(false, Demo.has_more_results(Demo::SearchResult.new(query: "rust ffi", total: 12, next_cursor: nil, max_score: nil)), "case:records.with_options.search_result.should_report_no_more_results_without_cursor")
  end
end
