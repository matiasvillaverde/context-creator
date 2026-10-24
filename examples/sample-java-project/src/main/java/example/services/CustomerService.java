package example.services;

import example.models.Customer;

public final class CustomerService {
    public Customer load() {
        return new Customer("Ada");
    }
}
