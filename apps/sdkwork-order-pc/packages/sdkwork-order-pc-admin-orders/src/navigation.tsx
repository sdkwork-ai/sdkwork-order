import {
  createContext,
  useContext,
  useMemo,
  type AnchorHTMLAttributes,
  type PropsWithChildren,
} from "react";

/**
 * Navigation port for order supervision screens.
 *
 * The package never depends on a router; screens render links through this
 * context. The default implementation is a plain anchor (full page
 * navigation, used by the standalone shell). Embedding hosts (e.g.
 * sdkwork-cloudrouter) inject their SPA `Link` component so in-app
 * navigation stays client-side.
 */
export type OrderAdminLinkProps = Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> & {
  href: string;
};

export type OrderAdminLinkComponent = (props: OrderAdminLinkProps) => React.ReactElement;

function DefaultOrderAdminLink({ href, children, ...rest }: OrderAdminLinkProps) {
  return (
    <a href={href} {...rest}>
      {children}
    </a>
  );
}

const OrderAdminLinkContext = createContext<OrderAdminLinkComponent>(DefaultOrderAdminLink);

export interface OrderAdminLinkProviderProps extends PropsWithChildren {
  /** SPA-aware link renderer injected by the embedding host. */
  linkComponent: OrderAdminLinkComponent;
}

export function OrderAdminLinkProvider({ children, linkComponent }: OrderAdminLinkProviderProps) {
  const value = useMemo(() => linkComponent, [linkComponent]);
  return (
    <OrderAdminLinkContext.Provider value={value}>
      {children}
    </OrderAdminLinkContext.Provider>
  );
}

export function useOrderAdminLink(): OrderAdminLinkComponent {
  return useContext(OrderAdminLinkContext);
}
