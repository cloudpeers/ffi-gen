export class Api {
  constructor();

  fetch(url, imports): Promise<void>;

  hello_world: void;

  async_hello_world: Promise;

  drop(): void;
}
