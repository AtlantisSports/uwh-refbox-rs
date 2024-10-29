module top(
    input clk,
    input rst_n,
    output bit [7:0] led,
    output bit [4:0] led_string,
    input usb_rx,
    output bit usb_tx
    );
    
    wire rst;
    
    reset_conditioner reset_conditioner(.clk(clk), .in(~rst_n), .out(rst));
    
    wire uart_byte_ready;
    wire [7:0] uart_byte;
    UART_RX #(.CLKS_PER_BIT(868)) uart_rx(.i_Rst_L(~rst), .i_Clock(clk), .i_RX_Serial(usb_rx), .o_RX_DV(uart_byte_ready), .o_RX_Byte(uart_byte[7:0]));

    // State machine states
    typedef enum logic [2:0] {
        IDLE,
        HEADER_RECEIVED,
        DATA_RECEIVED
    } state_t;
    
    // State machine signals
    state_t state;
    logic [7:0] data;
    // reg [31:0] timeout_counter;
    // parameter TIMEOUT_VALUE = 100000; // 1ms at 100MHz clock
    
    always_ff @(posedge clk or posedge rst) begin
        if (rst) begin
            state <= IDLE;
            // timeout_counter <= TIMEOUT_VALUE;
        end else begin
            case (state)
            IDLE:
                if (uart_byte_ready && uart_byte == 8'hAF) begin
                    state <= HEADER_RECEIVED;
                    led[7:6] <= 2'b10;
                    // timeout_counter <= TIMEOUT_VALUE;
                end
            HEADER_RECEIVED:
                if (uart_byte_ready) begin
                    data <= uart_byte;
                    state <= DATA_RECEIVED;
                    led[7:6] <= 2'b01;
                    // timeout_counter <= TIMEOUT_VALUE;
                end
            DATA_RECEIVED:
                if (uart_byte_ready) begin
                    if (uart_byte == 8'hBF) begin
                        led[5:0] <= data[5:0];
                    end
                    state <= IDLE;
                    led[7:6] <= 2'b00;
                    // timeout_counter <= TIMEOUT_VALUE;
                end
            endcase
            
            // if (timeout_counter == 0) begin
            //     state <= IDLE;
            //     led[7:6] <= 2'b11;
            // end else begin
            //     timeout_counter <= timeout_counter - 1;
            // end
        end
    end
    
endmodule
