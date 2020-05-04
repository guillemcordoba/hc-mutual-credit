import { LitElement } from 'lit-element';
import { Offer } from 'src/types';
import { ApolloClient } from 'apollo-boost';
declare const MCOfferDetail_base: {
    new (...args: any[]): import("@uprtcl/micro-orchestrator").ConnectedElement;
    prototype: any;
} & typeof LitElement;
export declare class MCOfferDetail extends MCOfferDetail_base {
    transactionId: string;
    offer: Offer;
    client: ApolloClient<any>;
    static get styles(): import("lit-element").CSSResult;
    firstUpdated(): Promise<void>;
    acceptOffer(): void;
    render(): import("lit-element").TemplateResult;
}
export {};