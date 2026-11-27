mod after_sales;
mod app_template;
mod checkout;
mod fulfillment;
mod management;
mod order_list_facets;
mod owner;
mod recharge;
mod shipment;

pub use app_template::{AppTemplateOrderListQuery, AppTemplateOrderPage, AppTemplateOrderSummary};
pub use order_list_facets::{collect_repeated_query_values, OrderListFacets};

pub use recharge::{
    AccountValueCatalogListQuery, AccountValueRequestDetailQuery, AccountValueRequestListQuery,
    CheckoutStatusQuery, RechargePackageListPage, RechargePackageListQuery, RechargeSettingsQuery,
};

pub use after_sales::{
    AfterSalesEventListQuery, AfterSalesEventPage, AfterSalesEventView,
    AfterSalesManagementDetailQuery, AfterSalesManagementListQuery, AfterSalesRequestDetailQuery,
    AfterSalesRequestListQuery, AfterSalesRequestPage, AfterSalesRequestView,
    AfterSalesReturnShipmentListQuery, AfterSalesReturnShipmentPage, AfterSalesReturnShipmentView,
    CreateAfterSalesRequestCommand, CreateAfterSalesRequestItemInput,
    CreateAfterSalesReturnShipmentCommand, ReviewAfterSalesRequestCommand,
    UpdateAfterSalesRequestCommand,
};
pub use checkout::{
    CheckoutLineInput, CheckoutQuoteView, CheckoutSessionDetailQuery, CheckoutSessionView,
    CreateCheckoutQuoteCommand, CreateCheckoutSessionCommand,
};
pub use fulfillment::{
    FulfillmentDetailQuery, FulfillmentListPage, FulfillmentListQuery,
    FulfillmentManagementListQuery, FulfillmentView,
};
pub use management::{
    AddOrderTagsCommand, CancelManagementOrderCommand, CloseManagementOrderCommand,
    OrderCancellationListQuery, OrderCancellationPage, OrderCancellationView,
    OrderClassificationView, OrderManagementDetailQuery, OrderManagementEventListQuery,
    OrderManagementEventPage, OrderManagementEventView, OrderManagementFacetView,
    OrderManagementListPage, OrderManagementListQuery, OrderSceneFacetView, OrderTagFacetView,
    ReplaceOrderClassificationCommand,
};
pub use owner::{
    CancelOwnerOrderCommand, CreateOwnerOrderCommand, CreateOwnerOrderOutcome, OrderOwnerDetail,
    OrderOwnerDetailQuery, OrderOwnerEventListQuery, OrderOwnerEventPage, OrderOwnerEventView,
    OrderOwnerItem, OrderOwnerListPage, OrderOwnerListQuery, OrderOwnerPaymentStatus,
    OrderOwnerStatistics, OrderOwnerSummary, PayOwnerOrderCommand, PayOwnerOrderCommandInput,
    PayOwnerOrderOutcome,
};
pub use shipment::{
    CarrierListQuery, CarrierPage, CarrierView, CreateCarrierCommand, CreateShipmentCommand,
    CreateShipmentPackageCommand, CreateShipmentTrackingEventCommand, MarkShipmentShippedCommand, ShipmentDetailQuery, ShipmentManagementDetailQuery,
    ShipmentManagementListPage, ShipmentManagementListQuery, ShipmentPackageListQuery,
    ShipmentPackageManagementListQuery, ShipmentPackagePage, ShipmentPackageView,
    ShipmentTrackingEventListQuery, ShipmentTrackingEventManagementListQuery,
    ShipmentTrackingEventPage, ShipmentTrackingEventView, ShipmentView, UpdateCarrierCommand,
    UpdateShipmentPackageCommand,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderListQuery {
    pub owner_user_id: String,
    pub status: Option<String>,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderDetailQuery {
    pub order_id: String,
    pub tenant_id: String,
}
