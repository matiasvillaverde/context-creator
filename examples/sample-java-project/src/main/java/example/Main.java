package example;

import example.services.CustomerService;

public final class Main {
    public static void main(String[] args) {
        CustomerService service = new CustomerService();
        System.out.println(service.load().name());
    }
}
