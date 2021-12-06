export class Api {
  constructor();

  fetch(url, imports): Promise<void>;

  helloWorld(): void;

  asyncHelloWorld(): Promise<number>;

  drop(): void;
}
