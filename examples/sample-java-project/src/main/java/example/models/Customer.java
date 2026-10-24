package example.models;

public final class Customer {
    private final String name;

    public Customer(String name) {
        this.name = name;
    }

    public String name() {
        return name;
    }
}
