# frozen_string_literal: true

class DefaultValueRecordTest < Minitest::Test
  def test_service_config_roundtrip
    # The Ruby record constructor requires every field; the binding does not render
    # Rust default values. Build the implicit-defaults shape explicitly.
    implicit_defaults = Demo::ServiceConfig.new(
      name: "worker", retries: 3, region: "standard", endpoint: "none", backup_endpoint: "https://default",
    )
    assert_equal(implicit_defaults, Demo.echo_service_config(implicit_defaults), "case:records.default_values.service_config.should_roundtrip_value")

    explicit_config = Demo::ServiceConfig.new(
      name: "worker", retries: 9, region: "eu-west", endpoint: "https://edge", backup_endpoint: "https://backup",
    )
    assert_equal(explicit_config, Demo.echo_service_config(explicit_config), "case:records.default_values.service_config.should_roundtrip_value")
  end
end
