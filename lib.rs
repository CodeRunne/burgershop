#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod burger_shop {

    use ink::env::debug_println;
    use ink::prelude::{format, vec::Vec};
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    /// Burger Type sold in the shop
    #[derive(Debug, Clone, Decode, Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum BurgerMenu {
        CheeseBurger,
        ChickenBurger,
        VeggieBurger,
    }

    /// Generate an implementation for the order struct
    impl BurgerMenu {
        /// Designate price for burger variants
        fn price(&self) -> Balance {
            match self {
                Self::CheeseBurger => 12,
                Self::VeggieBurger => 10,
                Self::ChickenBurger => 15,
            }
        }
    }

    /// Food sold in the shop
    #[derive(Debug, Clone, Decode, Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct FoodItem {
        burger_menu: BurgerMenu,
        amount: u32,
    }

    /// Generate an implementation for the fooditem struct
    impl FoodItem {
        /// Determine price for each food item in shop
        fn price(&self) -> Balance {
            match self.burger_menu {
                BurgerMenu::CheeseBurger => BurgerMenu::CheeseBurger.price() * self.amount as u128,
                BurgerMenu::ChickenBurger => {
                    BurgerMenu::ChickenBurger.price() * self.amount as u128
                }
                BurgerMenu::VeggieBurger => BurgerMenu::VeggieBurger.price() * self.amount as u128,
            }
        }
    }

    /// Order Struct. Contains the info of burgers ordered
    #[derive(Debug, Clone, Decode, Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Order {
        list_of_items: Vec<FoodItem>,
        customer: AccountId,
        total_price: Balance,
        paid: bool,
        order_id: u32,
    }

    /// Generate an implementation for the order struct
    impl Order {
        /// Initialize a new order
        fn new(list_of_items: Vec<FoodItem>, customer: AccountId, id: u32) -> Self {
            let total_price = Order::total_price(&list_of_items);
            Self {
                list_of_items,
                customer,
                total_price,
                paid: false,
                order_id: id,
            }
        }

        /// Get total price of the food items in the order book
        fn total_price(list_of_items: &Vec<FoodItem>) -> Balance {
            let mut total = 0;
            for item in list_of_items {
                total += item.price();
            }
            total
        }
    }

    /// Generate Events For Contract
    /// Transfer event, for when a transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    /// GetAllOrders Events, get emitted when the owner gets all orders in storage
    #[ink(event)]
    pub struct GetAllOrders {
        #[ink(topic)]
        orders: Vec<(u32, Order)>,
    }

    /// GetSingleOrder Event, gets emitted when owner gets a single order
    #[ink(event)]
    pub struct GetSingleOrder {
        #[ink(topic)]
        single_order: Order,
    }

    /// CreatedShopAndStorage
    #[ink(event)]
    pub struct CreatedShopAndStorage {
        #[ink(topic)]
        orders: Vec<(u32, Order)>,
    }

    /// Handle Errors that happens during operations
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    /// Errors types for different errors.
    pub enum BurgerShopError {
        PaymentError,
        OrderNotCompleted,
    }

    /// Result type
    pub type Result<T> = core::result::Result<T, BurgerShopError>;

    /// Contract storage for storing burger shop data
    #[ink(storage)]
    pub struct BurgerShop {
        orders: Vec<(u32, Order)>,
        orders_mapping: Mapping<u32, Order>,
    }

    /// Implements Burgershop contract storage struct
    impl BurgerShop {
        /// Initialize the burgershop with default/empty values
        #[ink(constructor)]
        pub fn new() -> Self {
            let order_storage_vector: Vec<(u32, Order)> = Vec::new();
            let order_storage_mapping = Mapping::new();

            Self {
                orders: order_storage_vector,
                orders_mapping: order_storage_mapping,
            }
        }

        /// Take order and make payment
        #[ink(message, payable)]
        pub fn take_order_and_payment(&mut self, list_of_items: Vec<FoodItem>) -> Result<Order> {
            // Get the caller account id
            let caller = Self::env().caller();

            // Assert the user is valid
            assert!(
                caller != self.env().account_id(),
                "You are not the customer!"
            );

            // assert the order contains at least 1 item
            assert!(list_of_items.len() as u32 > 0, "Can't take an empty order!");

            // Generate local id
            let id = self.orders.len() as u32;

            // Calculate and set order price
            let total_price = Order::total_price(&list_of_items);
            let mut order = Order::new(list_of_items, caller, id);
            order.total_price = total_price;

            // assert that the order hasn't been paid for already
            assert!(
                order.paid == false,
                "Can't pay for an order that is paid for already",
            );

            let multiply: Balance = 1_000_000_000_000;
            let transferred_val = self.env().transferred_value();

            // assert the value sent == total_price
            assert!(
                transferred_val
                    == order
                        .total_price
                        .checked_mul(multiply)
                        .expect("Overflow!!!"),
                "{}",
                format!("Please pay complete amount which is {}", order.total_price)
            );

            // print total price
            debug_println!("Expected value: {}", order.total_price);

            // print transferred_val
            debug_println!(
                "Expected received payment without conversion: {}",
                transferred_val
            );

            // make payment
            match self
                .env()
                .transfer(self.env().account_id(), order.total_price)
            {
                Ok(_) => {
                    // get current length of the list orders in storage
                    let id = self.orders.len() as u32;
                    order.paid = true;

                    // Emit event
                    self.env().emit_event(Transfer {
                        from: Some(order.customer),
                        to: Some(self.env().account_id()),
                        value: order.total_price,
                    });

                    // push to storage
                    self.orders_mapping.insert(id, &order);
                    self.orders.push((id, order.clone()));
                    Ok(order)
                }
                Err(_) => Err(BurgerShopError::PaymentError),
            }
        }

        /// Get a single order from storage
        #[ink(message)]
        pub fn get_single_order(&self, id: u32) -> Order {
            // get single order
            let order = self.orders_mapping.get(id).expect("Order not found");

            // emit event
            self.env().emit_event(GetSingleOrder {
                single_order: order.clone(),
            });

            // return order
            order
        }

        /// Get the orders in the storage
        #[ink(message)]
        pub fn get_orders(&self) -> Option<Vec<(u32, Order)>> {
            // Get all orders
            let get_all_orders = &self.orders;

            if get_all_orders.len() > 0 {
                let myorders: Vec<(u32, Order)> = get_all_orders.to_vec();

                // Emit events
                self.env().emit_event(GetAllOrders {
                    orders: myorders.clone(),
                });

                // converts reference to an owned/new vector
                Some(myorders)
            } else {
                None
            }
        }
    }
}
