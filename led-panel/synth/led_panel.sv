module led_panel(
    input clk,
    input rst_n,
    output bit [7:0] led,
    output bit ls_10a, ls_10b, ls_10c, ls_10d, ls_10e, ls_10f, ls_10g,
    output bit ls_1a, ls_1b, ls_1c, ls_1d, ls_1e, ls_1f, ls_1g,
    output bit rs_10a, rs_10b, rs_10c, rs_10d, rs_10e, rs_10f, rs_10g,
    output bit rs_1a, rs_1b, rs_1c, rs_1d, rs_1e, rs_1f, rs_1g,
    output bit white_on_left, white_on_right,
    output bit m_10a, m_10b, m_10c, m_10d, m_10e, m_10f, m_10g,
    output bit m_1a, m_1b, m_1c, m_1d, m_1e, m_1f, m_1g,
    output bit s_10a, s_10b, s_10c, s_10d, s_10e, s_10f, s_10g,
    output bit s_1a, s_1b, s_1c, s_1d, s_1e, s_1f, s_1g,
    output bit left_to_ind, right_to_ind, ref_to_ind,
    output bit one, slash, two, overtime, sdn_dth,
    output bit colon,
    input usb_rx,
    output bit usb_tx
    );
    
    wire rst;
    
    reset_conditioner reset_conditioner(.clk(clk), .in(!rst_n), .out(rst));

    logic timed_out = 1'b0;
    logic receiving = 1'b0;
    logic byte_rx = 1'b0;
    logic clrd = 1'b0;

    wire rx_complete_s, rx_complete_r, rx_complete;
    sr_ff rx_complete_ff(.clk(clk), .rst(rst), .s(rx_complete_s), .r(rx_complete_r), .q(rx_complete));

    wire disconnected_s, disconnected_r, disconnected;
    sr_ff disconnected_ff(.clk(clk), .rst(rst), .s(disconnected_s), .r(disconnected_r), .q(disconnected));

    wire data_ready;
    wire [7:0] data;
    UART_RX #(.CLKS_PER_BIT(868)) uart_rx(.i_Rst_L(!rst), .i_Clock(clk), .i_RX_Serial(usb_rx), .o_RX_DV(data_ready), .o_RX_Byte(data));

    localparam [4:0] 
        IDLE = 5'd0,
        DATA0 = 5'd1,
        DATA1 = 5'd2,
        DATA2 = 5'd3,
        DATA3 = 5'd4,
        DATA4 = 5'd5,
        DATA5 = 5'd6,
        DATA6 = 5'd7,
        DATA7 = 5'd8,
        DATA8 = 5'd9,
        DATA9 = 5'd10,
        DATA10 = 5'd11,
        DATA11 = 5'd12,
        DATA12 = 5'd13,
        DATA13 = 5'd14,
        DATA14 = 5'd15,
        DATA15 = 5'd16,
        DATA16 = 5'd17,
        DATA17 = 5'd18,
        DATA18 = 5'd19,
        DONE = 5'd20;

    reg [4:0] state = IDLE;
    logic [7:0] data_out [19:0];
    logic [7:0] data_q [19:0] = 160'h0;

    reg [15:0] timeout_counter = 16'b0;
    
    always @(posedge clk or posedge rst) begin
        if (rst) begin
            state <= IDLE;
            timeout_counter <= 16'b0;
            timed_out <= 1'b0;
            byte_rx <= 1'b0;
            receiving <= 1'b0;
            disconnected_r <= 1'b0;
            rx_complete_s <= 1'b0;
        end else begin
            if (disconnected) begin
                data_q <= 160'h0;
                disconnected_r <= 1'b1;
                clrd <= !clrd;
            end else begin
                disconnected_r <= 1'b0;
            end

            if (state == DONE) begin
                data_q <= data_out;
                rx_complete_s <= 1'b1;
                receiving <= 1'b0;
                state <= IDLE;
            end else begin
                rx_complete_s <= 1'b0;

                if (data_ready) begin
                    timeout_counter <= 16'b0;
                    byte_rx <= ~byte_rx;
                    case (state)
                        IDLE: begin
                            data_out[0] <= data;
                            timed_out <= 1'b0;
                            receiving <= 1'b1;
                            state <= DATA0;
                        end
                        DATA0: begin
                            data_out[1] <= data;
                            state <= DATA1;
                        end
                        DATA1: begin
                            data_out[2] <= data;
                            state <= DATA2;
                        end
                        DATA2: begin
                            data_out[3] <= data;
                            state <= DATA3;
                        end
                        DATA3: begin
                            data_out[4] <= data;
                            state <= DATA4;
                        end
                        DATA4: begin
                            data_out[5] <= data;
                            state <= DATA5;
                        end
                        DATA5: begin
                            data_out[6] <= data;
                            state <= DATA6;
                        end
                        DATA6: begin
                            data_out[7] <= data;
                            state <= DATA7;
                        end
                        DATA7: begin
                            data_out[8] <= data;
                            state <= DATA8;
                        end
                        DATA8: begin
                            data_out[9] <= data;
                            state <= DATA9;
                        end
                        DATA9: begin
                            data_out[10] <= data;
                            state <= DATA10;
                        end
                        DATA10: begin
                            data_out[11] <= data;
                            state <= DATA11;
                        end
                        DATA11: begin
                            data_out[12] <= data;
                            state <= DATA12;
                        end
                        DATA12: begin
                            data_out[13] <= data;
                            state <= DATA13;
                        end
                        DATA13: begin
                            data_out[14] <= data;
                            state <= DATA14;
                        end
                        DATA14: begin
                            data_out[15] <= data;
                            state <= DATA15;
                        end
                        DATA15: begin
                            data_out[16] <= data;
                            state <= DATA16;
                        end
                        DATA16: begin
                            data_out[17] <= data;
                            state <= DATA17;
                        end
                        DATA17: begin
                            data_out[18] <= data;
                            state <= DATA18;
                        end
                        DATA18: begin
                            data_out[19] <= data;
                            state <= DONE;
                        end
                    endcase
                end else if (timeout_counter >= 16'd34720) begin
                    receiving <= 1'b0;
                    state <= IDLE;
                    timed_out <= 1'b1;
                end else if (state != IDLE) begin
                    timeout_counter <= timeout_counter + 1;
                end
            end
        end
    end

    logic dcon = 1'b0;

    reg [25:0] disconenct_counter_d = 26'b0;
    reg [25:0] disconenct_counter_q = 26'b0;
    always @(posedge clk or posedge rst) begin
        if (rst) begin
            disconenct_counter_d <= 26'b0;
            disconenct_counter_q <= 26'b0;
            rx_complete_r <= 1'b0;
            disconnected_s <= 1'b0;
        end else begin
            disconenct_counter_q <= disconenct_counter_d;

            if (rx_complete) begin
                disconenct_counter_d <= 26'b0;
                rx_complete_r <= 1'b1;
            end else begin
                rx_complete_r <= 1'b0;

                if (disconenct_counter_q >= 26'd50000000) begin
                    // We haven't received any data in 0.5 seconds (we should have received 5
                    // messages), so we clear the display until a new message is received
                    disconnected_s <= 1'b1;
                    dcon <= !dcon;
                    disconenct_counter_d <= 26'b0;
                end else begin
                    disconnected_s <= 1'b0;
                    disconenct_counter_d <= disconenct_counter_q + 1;
                end
            end
        end
    end

    digit ls_10, ls_1, rs_10, rs_1, m_10, m_1, s_10, s_1;
    wire white_on_left_en, white_on_right_en;
    wire left_to_ind_en, right_to_ind_en, ref_to_ind_en;
    wire one_en, slash_en, two_en, overtime_en, sdn_dth_en;
    wire colon_en;
    wire [1:0] brightness;
    segments segments(
        .data(data_q),
        .ls_10(ls_10), .ls_1(ls_1), .rs_10(rs_10), .rs_1(rs_1), .m_10(m_10), .m_1(m_1), .s_10(s_10), .s_1(s_1),
        .white_on_left(white_on_left_en), .white_on_right(white_on_right_en),
        .left_to_ind(left_to_ind_en), .right_to_ind(right_to_ind_en), .ref_to_ind(ref_to_ind_en),
        .one(one_en), .slash(slash_en), .two(two_en), .overtime(overtime_en), .sdn_dth(sdn_dth_en),
        .colon(colon_en),
        .brightness(brightness)
    );

    reg [10:0] pwm_counter = 11'b0;
    wire pwm_clk;

    assign pwm_clk = pwm_counter[10];

    always @(posedge clk or posedge rst) begin
        if (rst) begin
            pwm_counter <= 11'b0;
        end else begin
            pwm_counter <= pwm_counter + 1;
        end
    end

    wire [66:0] pwm_out;

    // Spacing 3
    reg [7:0] compare_low [67:0] = {8'hC9, 8'hC6, 8'hC3, 8'hC0, 8'hBD, 8'hBA, 8'hB7, 8'hB4, 8'hB1, 8'hAE, 8'hAB, 8'hA8, 8'hA5, 8'hA2, 8'h9F, 8'h9C, 8'h99, 8'h96, 8'h93, 8'h90, 8'h8D, 8'h8A, 8'h87, 8'h84, 8'h81, 8'h7E, 8'h7B, 8'h78, 8'h75, 8'h72, 8'h6F, 8'h6C, 8'h69, 8'h66, 8'h63, 8'h60, 8'h5D, 8'h5A, 8'h57, 8'h54, 8'h51, 8'h4E, 8'h4B, 8'h48, 8'h45, 8'h42, 8'h3F, 8'h3C, 8'h39, 8'h36, 8'h33, 8'h30, 8'h2D, 8'h2A, 8'h27, 8'h24, 8'h21, 8'h1E, 8'h1B, 8'h18, 8'h15, 8'h12, 8'h0F, 8'h0C, 8'h09, 8'h06, 8'h03, 8'h00};
    wire [66:0] pwm_int_low;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(67)) pwm_low(.clk(pwm_clk), .rst(rst), .compare(compare_low), .pwm(pwm_int_low));

    // Spacing 10
    reg [7:0] compare_med [25:0] = {8'hFA, 8'hF0, 8'hE6, 8'hDC, 8'hD2, 8'hC8, 8'hBE, 8'hB4, 8'hAA, 8'hA0, 8'h96, 8'h8C, 8'h82, 8'h78, 8'h6E, 8'h64, 8'h5A, 8'h50, 8'h46, 8'h3C, 8'h32, 8'h28, 8'h1E, 8'h14, 8'h0A, 8'h00};
    wire [24:0] pwm_int_med;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(25)) pwm_med(.clk(pwm_clk), .rst(rst), .compare(compare_med), .pwm(pwm_int_med));

    // Spacing 21
    reg [7:0] compare_high [12:0] = {8'hFC, 8'hE7, 8'hD2, 8'hBD, 8'hA8, 8'h93, 8'h7E, 8'h69, 8'h54, 8'h3F, 8'h2A, 8'h15, 8'h00};
    wire [11:0] pwm_int_high;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(12)) pwm_high(.clk(pwm_clk), .rst(rst), .compare(compare_high), .pwm(pwm_int_high));

    // Spacing 36
    reg [7:0] compare_outdoor [7:0] = {8'hFC, 8'hD8, 8'hB4, 8'h90, 8'h6C, 8'h48, 8'h24, 8'h00};
    wire [6:0] pwm_int_outdoor;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(7)) pwm_outdoor(.clk(pwm_clk), .rst(rst), .compare(compare_outdoor), .pwm(pwm_int_outdoor));

    assign pwm_out = (brightness == 2'b00) ? pwm_int_low :
                     (brightness == 2'b01) ? {pwm_int_med[16:0], pwm_int_med[24:0], pwm_int_med[24:0]} :
                     (brightness == 2'b10) ? {pwm_int_high[6:0], pwm_int_high[11:0], pwm_int_high[11:0], pwm_int_high[11:0], pwm_int_high[11:0], pwm_int_high[11:0]} :
                     {pwm_int_outdoor[3:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0], pwm_int_outdoor[6:0]};

    assign ls_10a = ls_10.a && pwm_out[0];
    assign ls_10b = ls_10.b && pwm_out[1];
    assign ls_10c = ls_10.c && pwm_out[2];
    assign ls_10d = ls_10.d && pwm_out[3];
    assign ls_10e = ls_10.e && pwm_out[4];
    assign ls_10f = ls_10.f && pwm_out[5];
    assign ls_10g = ls_10.g && pwm_out[6];

    assign ls_1a = ls_1.a && pwm_out[7];
    assign ls_1b = ls_1.b && pwm_out[8];
    assign ls_1c = ls_1.c && pwm_out[9];
    assign ls_1d = ls_1.d && pwm_out[10];
    assign ls_1e = ls_1.e && pwm_out[11];
    assign ls_1f = ls_1.f && pwm_out[12];
    assign ls_1g = ls_1.g && pwm_out[13];

    assign rs_10a = rs_10.a && pwm_out[14];
    assign rs_10b = rs_10.b && pwm_out[15];
    assign rs_10c = rs_10.c && pwm_out[16];
    assign rs_10d = rs_10.d && pwm_out[17];
    assign rs_10e = rs_10.e && pwm_out[18];
    assign rs_10f = rs_10.f && pwm_out[19];
    assign rs_10g = rs_10.g && pwm_out[20];

    assign rs_1a = rs_1.a && pwm_out[21];
    assign rs_1b = rs_1.b && pwm_out[22];
    assign rs_1c = rs_1.c && pwm_out[23];
    assign rs_1d = rs_1.d && pwm_out[24];
    assign rs_1e = rs_1.e && pwm_out[25];
    assign rs_1f = rs_1.f && pwm_out[26];
    assign rs_1g = rs_1.g && pwm_out[27];

    assign white_on_left = white_on_left_en && pwm_out[28];
    assign white_on_right = white_on_right_en && pwm_out[29];

    assign m_10a = m_10.a && pwm_out[30];
    assign m_10b = m_10.b && pwm_out[31];
    assign m_10c = m_10.c && pwm_out[32];
    assign m_10d = m_10.d && pwm_out[33];
    assign m_10e = m_10.e && pwm_out[34];
    assign m_10f = m_10.f && pwm_out[35];
    assign m_10g = m_10.g && pwm_out[36];

    assign m_1a = m_1.a && pwm_out[37];
    assign m_1b = m_1.b && pwm_out[38];
    assign m_1c = m_1.c && pwm_out[39];
    assign m_1d = m_1.d && pwm_out[40];
    assign m_1e = m_1.e && pwm_out[41];
    assign m_1f = m_1.f && pwm_out[42];
    assign m_1g = m_1.g && pwm_out[43];

    assign s_10a = s_10.a && pwm_out[44];
    assign s_10b = s_10.b && pwm_out[45];
    assign s_10c = s_10.c && pwm_out[46];
    assign s_10d = s_10.d && pwm_out[47];
    assign s_10e = s_10.e && pwm_out[48];
    assign s_10f = s_10.f && pwm_out[49];
    assign s_10g = s_10.g && pwm_out[50];

    assign s_1a = s_1.a && pwm_out[51];
    assign s_1b = s_1.b && pwm_out[52];
    assign s_1c = s_1.c && pwm_out[53];
    assign s_1d = s_1.d && pwm_out[54];
    assign s_1e = s_1.e && pwm_out[55];
    assign s_1f = s_1.f && pwm_out[56];
    assign s_1g = s_1.g && pwm_out[57];

    assign left_to_ind = left_to_ind_en && pwm_out[58];
    assign right_to_ind = right_to_ind_en && pwm_out[59];
    assign ref_to_ind = ref_to_ind_en && pwm_out[60];

    assign one = one_en && pwm_out[61];
    assign slash = slash_en && pwm_out[62];
    assign two = two_en && pwm_out[63];
    assign overtime = overtime_en && pwm_out[64];
    assign sdn_dth = sdn_dth_en && pwm_out[65];

    assign colon = colon_en && pwm_out[66];

    assign led[0] = rx_complete;
    assign led[1] = usb_rx;
    assign led[2] = pwm_clk;
    assign led[3] = pwm_int_low[0];
    assign led[4] = disconenct_counter_q[25];
    assign led[5] = disconenct_counter_q[0];
    assign led[7:6] = brightness;

endmodule
