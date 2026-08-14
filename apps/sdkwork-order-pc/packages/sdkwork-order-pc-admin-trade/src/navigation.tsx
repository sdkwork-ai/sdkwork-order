import {
  createContext,
  useContext,
  useMemo,
  type AnchorHTMLAttributes,
  type PropsWithChildren,
} from "react";

/**
 * Navigation port for trading center screens.
 *
 * The package never depends on a router; screens render links through this
 * context. The default implementation is a plain anchor (full page
 * navigation, used by the standalone shell). Embedding hosts (e.g.
 * sdkwork-cloudrouter) inject their SPA `Link` component so in-app
 * navigation stays client-side.
 */
export type TradeAdminLinkProps = Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> & {
  href: string;
};

export type TradeAdminLinkComponent = (props: TradeAdminLinkProps) => React.ReactElement;

function DefaultTradeAdminLink({ href, children, ...rest }: TradeAdminLinkProps) {
  return (
    <a href={href} {...rest}>
      {children}
    </a>
  );
}

const TradeAdminLinkContext = createContext<TradeAdminLinkComponent>(DefaultTradeAdminLink);

export interface TradeAdminLinkProviderProps extends PropsWithChildren {
  /** SPA-aware link renderer injected by the embedding host. */
  linkComponent: TradeAdminLinkComponent;
}

export function TradeAdminLinkProvider({ children, linkComponent }: TradeAdminLinkProviderProps) {
  const value = useMemo(() => linkComponent, [linkComponent]);
  return (
    <TradeAdminLinkContext.Provider value={value}>
      {children}
    </TradeAdminLinkContext.Provider>
  );
}

export function useTradeAdminLink(): TradeAdminLinkComponent {
  return useContext(TradeAdminLinkContext);
}
