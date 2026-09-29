# frozen_string_literal: true

class StringRecordTest < Minitest::Test
  def test_person
    person = Demo.make_person("Ali", 30)
    assert_equal(Demo::Person.new(name: "Ali", age: 30), person, "case:records.with_strings.person.should_make_from_fields")
    assert_equal(person, Demo.echo_person(person), "case:records.with_strings.person.should_roundtrip_value")
    assert_equal("Hello, Ali! You are 30 years old.", Demo.greet_person(person), "case:records.with_strings.person.should_format_greeting")
  end

  def test_address
    address = Demo::Address.new(street: "Main St", city: "Amsterdam", zip: "1000AA")
    assert_equal(address, Demo.echo_address(address), "case:records.with_strings.address.should_roundtrip_value")
    assert_equal("Main St, Amsterdam, 1000AA", Demo.format_address(address), "case:records.with_strings.address.should_format_value")
  end
end
