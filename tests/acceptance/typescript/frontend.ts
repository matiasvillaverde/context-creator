// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

import { trace, context, SpanStatusCode } from '@opentelemetry/api';
import { SemanticAttributes } from '@opentelemetry/semantic-conventions';

const tracer = trace.getTracer('frontend-service');

export interface Product {
  id: string;
  name: string;
  price: number;
  currency: string;
  imageUrl: string;
}

export interface CartItem {
  productId: string;
  quantity: number;
}

export interface Cart {
  userId: string;
  items: CartItem[];
}

export class FrontendService {
  private apiBaseUrl: string;

  constructor(apiBaseUrl: string = 'http://localhost:8080') {
    this.apiBaseUrl = apiBaseUrl;
  }

  /**
   * Get list of products from catalog
   */
  async getProducts(currency: string = 'USD'): Promise<Product[]> {
    const span = tracer.startSpan('getProducts', {
      attributes: {
        'currency': currency,
        'http.method': 'GET',
        'http.url': `${this.apiBaseUrl}/api/products`
      }
    });

    return context.with(trace.setSpan(context.active(), span), async () => {
      try {
        const response = await fetch(`${this.apiBaseUrl}/api/products?currency=${currency}`);
        
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }

        const products: Product[] = await response.json();
        span.setAttribute('product.count', products.length);
        
        return products;
      } catch (error) {
        span.recordException(error as Error);
        span.setStatus({ code: SpanStatusCode.ERROR, message: (error as Error).message });
        throw error;
      } finally {
        span.end();
      }
    });
  }

  /**
   * Get single product details
   */
  async getProduct(productId: string): Promise<Product> {
    const span = tracer.startSpan('getProduct', {
      attributes: {
        'product.id': productId,
        'http.method': 'GET',
        'http.url': `${this.apiBaseUrl}/api/products/${productId}`
      }
    });

    return context.with(trace.setSpan(context.active(), span), async () => {
      try {
        const response = await fetch(`${this.apiBaseUrl}/api/products/${productId}`);
        
        if (!response.ok) {
          if (response.status === 404) {
            span.setAttribute('product.found', false);
            throw new Error('Product not found');
          }
          throw new Error(`HTTP error! status: ${response.status}`);
        }

        const product: Product = await response.json();
        span.setAttributes({
          'product.name': product.name,
          'product.price': product.price,
          'product.found': true
        });
        
        return product;
      } catch (error) {
        span.recordException(error as Error);
        span.setStatus({ code: SpanStatusCode.ERROR, message: (error as Error).message });
        throw error;
      } finally {
        span.end();
      }
    });
  }

  /**
   * Add item to cart
   */
  async addToCart(userId: string, productId: string, quantity: number): Promise<void> {
    const span = tracer.startSpan('addToCart', {
      attributes: {
        'user.id': userId,
        'product.id': productId,
        'cart.quantity': quantity,
        'http.method': 'POST'
      }
    });

    return context.with(trace.setSpan(context.active(), span), async () => {
      try {
        // Validate quantity
        if (quantity <= 0) {
          throw new Error('Quantity must be greater than 0');
        }

        const response = await fetch(`${this.apiBaseUrl}/api/cart`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-User-Id': userId
          },
          body: JSON.stringify({
            productId,
            quantity
          })
        });

        if (!response.ok) {
          throw new Error(`Failed to add to cart: ${response.status}`);
        }

        span.setAttribute('cart.operation.success', true);
      } catch (error) {
        span.recordException(error as Error);
        span.setStatus({ code: SpanStatusCode.ERROR, message: (error as Error).message });
        throw error;
      } finally {
        span.end();
      }
    });
  }

  /**
   * Get current cart contents
   */
  async getCart(userId: string): Promise<Cart> {
    const span = tracer.startSpan('getCart', {
      attributes: {
        'user.id': userId,
        'http.method': 'GET'
      }
    });

    return context.with(trace.setSpan(context.active(), span), async () => {
      try {
        const response = await fetch(`${this.apiBaseUrl}/api/cart`, {
          headers: {
            'X-User-Id': userId
          }
        });

        if (!response.ok) {
          throw new Error(`Failed to get cart: ${response.status}`);
        }

        const cart: Cart = await response.json();
        span.setAttributes({
          'cart.items.count': cart.items.length,
          'cart.empty': cart.items.length === 0
        });

        return cart;
      } catch (error) {
        span.recordException(error as Error);
        span.setStatus({ code: SpanStatusCode.ERROR, message: (error as Error).message });
        throw error;
      } finally {
        span.end();
      }
    });
  }

  /**
   * Checkout the current cart
   */
  async checkout(userId: string, paymentInfo: any): Promise<string> {
    const span = tracer.startSpan('checkout', {
      attributes: {
        'user.id': userId,
        'http.method': 'POST'
      }
    });

    return context.with(trace.setSpan(context.active(), span), async () => {
      try {
        // First get the cart to validate
        const cart = await this.getCart(userId);
        
        if (cart.items.length === 0) {
          throw new Error('Cannot checkout empty cart');
        }

        span.setAttribute('checkout.items.count', cart.items.length);

        const response = await fetch(`${this.apiBaseUrl}/api/checkout`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-User-Id': userId
          },
          body: JSON.stringify(paymentInfo)
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`Checkout failed: ${errorText}`);
        }

        const result = await response.json();
        const orderId = result.orderId;
        
        span.setAttributes({
          'order.id': orderId,
          'checkout.success': true
        });

        return orderId;
      } catch (error) {
        span.recordException(error as Error);
        span.setStatus({ code: SpanStatusCode.ERROR, message: (error as Error).message });
        throw error;
      } finally {
        span.end();
      }
    });
  }
}