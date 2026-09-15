# frozen_string_literal: true

module Mechanomeld
  # A typed Automerge value that has no natural Ruby equivalent.
  class Scalar
    attr_reader :value

    def initialize(value)
      @value = value
    end

    def ==(other)
      other.class == self.class && other.value == value
    end
    alias_method :eql?, :==

    def hash
      [self.class, value].hash
    end

    def inspect
      "#<#{self.class.name} #{value.inspect}>"
    end
  end

  class Counter < Scalar; end

  # Milliseconds since the Unix epoch.
  class Timestamp < Scalar; end

  class Uint < Scalar; end

  class Bytes < Scalar
    def initialize(value)
      super(String(value).b)
    end
  end

  # A collaborative text object, as opposed to a plain String value.
  class Text < Scalar
    def initialize(value = "")
      super(String(value))
    end

    def to_s
      value
    end
  end
end
